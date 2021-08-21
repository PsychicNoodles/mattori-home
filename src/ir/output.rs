use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use color_eyre::eyre::WrapErr;
use eyre::Result;
use rppal::gpio::{Gpio, PwmPulse, PwmStep};
use tokio::sync::watch;
use tokio::task::spawn_blocking;

use crate::ir::types::{IrSequence, IrTarget};
use core::iter;

const WAIT_TIMEOUT: Duration = Duration::from_micros(100);

pub struct IrOut<T: 'static + IrTarget> {
    target: T,
    sequence_sender: mpsc::Sender<IrSequence>,
    send_stop_sender: watch::Sender<bool>,
}

impl<T: 'static + IrTarget> IrOut<T> {
    pub fn start(pin: u8, target: T) -> Result<IrOut<T>> {
        let out = Arc::new(Mutex::new(
            Gpio::new()
                .wrap_err("Could not initialize gpio")?
                .get(pin)
                .wrap_err_with(|| format!("Could not get gpio pin {}", pin))?
                .into_output(),
        ));
        let (send_stop_sender, send_stop_receiver) = watch::channel(false);
        let (sequence_sender, sequence_receiver) = mpsc::channel::<IrSequence>();
        spawn_blocking(move || loop {
            if *send_stop_receiver.borrow() {
                trace!("stopping ir sender thread");
                break;
            }

            match sequence_receiver.recv_timeout(WAIT_TIMEOUT) {
                Ok(seq) => {
                    let out = out.clone();
                    spawn_blocking(move || match out.lock() {
                        Err(_) => {
                            error!("Could not get lock for ir output!");
                        }
                        Ok(mut o) => {
                            if let Err(e) = o.set_pwm_sequence(
                                seq.into_inner().into_iter().enumerate().fold(
                                    Vec::new(),
                                    |mut acc, (i, pulse)| {
                                        if i % 2 == 0 {
                                            acc.extend(
                                                iter::repeat(PwmStep::Pulse(PwmPulse {
                                                    period: Duration::from_micros(18),
                                                    pulse_width: Duration::from_micros(8),
                                                }))
                                                .take((pulse.into_inner() / 26) as usize),
                                            );
                                        } else {
                                            acc.push(PwmStep::Wait(Duration::from_micros(
                                                pulse.0 as u64,
                                            )));
                                        }
                                        acc
                                    },
                                ),
                                false,
                            ) {
                                error!("Could not set up pwm for ir output: {:?}", e);
                            }
                        }
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // nothing from seq receiver for a bit, so loop to check if stop received
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    info!("ir sequence sender disconnected before stop signal");
                    break;
                }
            }
        });
        Ok(IrOut {
            target,
            sequence_sender,
            send_stop_sender,
        })
    }

    pub fn send(&self, seq: IrSequence) -> Result<()> {
        debug!("sending sequence: {:?}", seq);
        self.sequence_sender
            .send(seq)
            .wrap_err("Tried to send ir sequence to sender thread")
    }

    pub fn stop(&mut self) -> Result<()> {
        self.send_stop_sender
            .send(true)
            .wrap_err("Could not send stop to ir sequence sender")
    }

    pub fn send_target<F: FnMut(&mut T) -> Result<IrSequence, T::Error>>(
        &mut self,
        mut action: F,
    ) -> Result<()> {
        let sequence = action(&mut self.target)?;
        self.send(sequence)
    }
}
