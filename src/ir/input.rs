use crate::ir::types::{IrPulse, IrPulseError};
use async_stream::try_stream;
use eyre::{eyre, Result, WrapErr};
use futures::Stream;
use rppal::gpio::{Gpio, Level};
use std::convert::TryFrom;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use std::{sync::mpsc, thread::sleep};
use tokio::{
    sync::watch,
    task::{spawn_blocking, JoinHandle},
};

pub type PulseSequence = Arc<Vec<IrPulse>>;

const WAIT_TIMEOUT: Duration = Duration::from_micros(100);

pub struct IrIn {
    read_handle: JoinHandle<()>,
    read_stop_sender: watch::Sender<bool>,
    pulses: Arc<RwLock<Vec<PulseSequence>>>,
    pulse_added_receiver: watch::Receiver<Option<PulseSequence>>,
}

impl IrIn {
    pub fn start(pin: u8) -> Result<IrIn> {
        let ir = Gpio::new()
            .wrap_err("Could not initialize gpio")?
            .get(pin)
            .wrap_err_with(|| format!("Could not get gpio pin {}", pin))?
            .into_input();
        let (read_stop_sender, read_stop_receiver) = watch::channel(false);
        let pulses = Arc::new(RwLock::new(Vec::new()));
        let (pulse_added_sender, pulse_added_receiver) = watch::channel(None);
        let read_handle = {
            let pulses = pulses.clone();
            spawn_blocking(move || {
                let (ir_input_sender, ir_input_receiver) = mpsc::channel();
                spawn_blocking(move || loop {
                    if let Err(_) = ir_input_sender.send(ir.read()) {
                        info!("ir input reader closed");
                        break;
                    }
                    sleep(Duration::from_micros(1));
                });

                let mut sequence = Vec::new();
                let mut last = None;
                let mut count = 0;
                loop {
                    if *read_stop_receiver.borrow() {
                        trace!("stopping ir receiver thread");
                        break;
                    }

                    match ir_input_receiver.recv_timeout(WAIT_TIMEOUT) {
                        Ok(pulse) => match last {
                            Some(lst) if lst == pulse => {
                                count += 1;
                                if count > IrPulse::MAX_WIDTH {
                                    // pulse is too long, so must be end of sequence
                                    if sequence.is_empty() {
                                        // no pulses yet so must be waiting for input
                                    } else {
                                        match pulses.write() {
                                            Err(_) => {
                                                error!(
                                                    "could not get write lock for pulses vector"
                                                );
                                                break;
                                            }
                                            Ok(mut lock) => {
                                                trace!("finished sequence {:?}", sequence);
                                                let finished_sequence = Arc::new(sequence.clone());
                                                lock.push(finished_sequence.clone());
                                                if let Err(e) =
                                                    pulse_added_sender.send(Some(finished_sequence))
                                                {
                                                    error!(
                                                    "could not send to pulse added sender: {:?}",
                                                    e
                                                );
                                                }
                                                sequence.clear();
                                            }
                                        }
                                    }
                                }
                            }
                            Some(_) => {
                                if pulse == Level::High {
                                    // end of Low pulse
                                    if !IrPulse::is_valid_low(count) {
                                        error!("unknown low ir pulse width ({})", count);
                                    }
                                    count = 1;
                                } else {
                                    // end of High pulse
                                    match IrPulse::try_from(count) {
                                        Ok(value) => {
                                            trace!("adding to sequence {:?}", value);
                                            sequence.push(value);
                                        }
                                        Err(IrPulseError::Zero) => {
                                            info!(
                                            "zero length ir pulse, probably initial reader setup"
                                        );
                                        }
                                        Err(IrPulseError::TooLong) => {
                                            error!("too long ir pulse, possibly gap between button presses ({})", count);
                                        }
                                        Err(IrPulseError::UnknownWidth) => {
                                            error!("unknown ir pulse width ({})", count);
                                        }
                                    }
                                    count = 0;
                                }
                                last = Some(pulse);
                            }
                            None => {
                                trace!("initial reader setup");
                                last = Some(pulse);
                            }
                        },
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            // nothing from the ir for a bit, so loop to check if stop received
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            info!("ir input reader disconnected before processing thread");
                            break;
                        }
                    }
                }
            })
        };
        Ok(IrIn {
            read_handle,
            read_stop_sender,
            pulses,
            pulse_added_receiver,
        })
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.read_stop_sender
            .send(true)
            .wrap_err("Could not send stop to ir reader")?;
        (&mut self.read_handle)
            .await
            .wrap_err("Could not wait for read thread to stop")
    }

    pub fn pulses(&self) -> Result<RwLockReadGuard<Vec<PulseSequence>>> {
        self.pulses
            .read()
            .map_err(|_| eyre!("Tried to acquire read lock to pulses vector"))
    }

    pub fn pulses_mut(&mut self) -> Result<RwLockWriteGuard<Vec<PulseSequence>>> {
        self.pulses
            .write()
            .map_err(|_| eyre!("Tried to acquire write lock to pulses vector"))
    }

    pub fn pulse_stream(&self) -> impl Stream<Item = Result<Option<PulseSequence>>> {
        let mut receiver = self.pulse_added_receiver.clone();
        try_stream! {
            loop {
                receiver.changed().await.wrap_err("Tried getting next pulse sequence")?;
                yield receiver.borrow().clone();
            }
        }
    }
}
