use std::convert::TryFrom;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use async_stream::{stream, try_stream};
use eyre::{eyre, Result, WrapErr};
use futures::Stream;
use rppal::gpio::{Gpio, InputPin, Level, Trigger};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{broadcast, mpsc, Notify, OnceCell};
use tokio::time::sleep;
use tokio::{
    pin,
    sync::watch,
    task::{spawn, spawn_blocking, JoinHandle},
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

use crate::ir::types::IrPulse;
use num_traits::PrimInt;

pub type IrPulseSequence = Arc<Vec<IrPulse>>;

const WAIT_TIMEOUT: Duration = Duration::from_millis(100);
const DEBOUNCE: Duration = Duration::from_micros(100);
const MAX_PULSE: Duration = Duration::from_millis(10);

pub struct IrIn {
    read_handle: JoinHandle<()>,
    read_stop_sender: watch::Sender<bool>,
    pulses: Arc<RwLock<Vec<IrPulseSequence>>>,
    pulse_added_receiver: watch::Receiver<Option<IrPulseSequence>>,
}

#[derive(Debug, Clone)]
enum IrInterruptMessage {
    Pulse(Duration),
    Timeout,
}

impl IrIn {
    pub fn start(pin: u8) -> Result<IrIn> {
        let mut ir = Gpio::new()
            .wrap_err("Could not initialize gpio")?
            .get(pin)
            .wrap_err_with(|| format!("Could not get gpio pin {}", pin))?
            .into_input();
        let (read_stop_sender, read_stop_receiver) = watch::channel(false);
        let pulses = Arc::new(RwLock::new(Vec::new()));
        let (pulse_added_sender, pulse_added_receiver) = watch::channel(None);
        let read_handle = {
            let pulses = pulses.clone();
            spawn(async move {
                let (ir_pulse_sender, ir_pulse_receiver) = mpsc::unbounded_channel();
                let timeout_handle =
                    match Self::start_ir_interrupt_handler(&mut ir, ir_pulse_sender) {
                        Err(e) => {
                            error!("failed to start ir interrupt handler: {:?}", e);
                            return;
                        }
                        Ok(h) => h,
                    };
                pin! {
                    let ir_pulse_stream = Self::debounce(UnboundedReceiverStream::new(ir_pulse_receiver)).map(Self::normalize);
                }

                let mut sequence = Vec::new();
                loop {
                    if *read_stop_receiver.borrow() {
                        trace!("stopping ir receiver thread");
                        break;
                    }

                    match ir_pulse_stream.next().await {
                        Some(IrInterruptMessage::Pulse(duration)) => {
                            if duration > MAX_PULSE {
                                info!("pulse duration is huge ({}ms), probably from waiting for signal so skipping", duration.as_micros());
                            } else {
                                sequence.push(IrPulse(duration.as_micros()));
                            }
                        }
                        Some(IrInterruptMessage::Timeout) => {
                            if !sequence.is_empty() {
                                match pulses.write() {
                                    Err(_) => {
                                        error!("could not get write lock for pulses vector");
                                        break;
                                    }
                                    Ok(mut lock) => {
                                        trace!("finished sequence {:?}", sequence);
                                        let finished_sequence = Arc::new(sequence.clone());
                                        lock.push(finished_sequence.clone());
                                        if let Err(e) =
                                            pulse_added_sender.send(Some(finished_sequence))
                                        {
                                            error!("could not send to pulse added sender: {:?}", e);
                                        }
                                        sequence.clear();
                                    }
                                }
                            }
                        }
                        None => {
                            info!("ir input reader disconnected before processing thread");
                            break;
                        }
                    }
                }
                timeout_handle.abort();
                if let Err(e) = ir.clear_async_interrupt() {
                    error!("could not clear ir interrupt handler: {:?}", e);
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

    fn start_ir_interrupt_handler(
        mut ir: &mut InputPin,
        ir_pulse_sender: UnboundedSender<IrInterruptMessage>,
    ) -> Result<JoinHandle<()>> {
        let mut last_inst = Instant::now();
        let mut last_level = Level::Low;
        let timeout_reset_notify = Arc::new(Notify::new());
        let timeout_handle = {
            let timeout_sender = ir_pulse_sender.clone();
            let timeout_reset_notify = timeout_reset_notify.clone();
            spawn(async move {
                // wait for start from interrupt handler
                timeout_reset_notify.notified().await;
                loop {
                    tokio::select! {
                        _ = sleep(WAIT_TIMEOUT) => {
                            if let Err(_) = timeout_sender.send(IrInterruptMessage::Timeout) {
                                info!("ir input timeout sender closed unexpectedly");
                            }
                        },
                        _ = timeout_reset_notify.notified() => {
                            trace!("timeout reset");
                        }
                    }
                }
            })
        };

        let mut init = true;
        ir.set_async_interrupt(Trigger::Both, move |level| {
            let now = Instant::now();

            if let Err(_) =
                ir_pulse_sender.send(IrInterruptMessage::Pulse(now.duration_since(last_inst)))
            {
                info!("ir input reader closed");
            }

            last_inst = now;
            last_level = level;
            if init {
                timeout_reset_notify.notify_one();
            }
        })
        .wrap_err("Could not set up ir interrupt handler")?;
        Ok(timeout_handle)
    }

    fn debounce<S: Stream<Item = IrInterruptMessage> + Unpin>(
        mut input_stream: S,
    ) -> impl Stream<Item = IrInterruptMessage> {
        stream! {
            let mut last: Option<Duration> = None;
            while let Some(input) = input_stream.next().await {
                match input {
                    IrInterruptMessage::Timeout => {
                        if let Some(l) = last {
                            yield IrInterruptMessage::Pulse(l);
                            last = None;
                        }
                        yield IrInterruptMessage::Timeout;
                    },
                    IrInterruptMessage::Pulse(duration) => {
                        match last.as_mut() {
                            Some(l) if *l + duration > DEBOUNCE => {
                                yield IrInterruptMessage::Pulse(*l + duration);
                                last = None;
                            },
                            Some(l) => {
                                *l += duration;
                            },
                            None if duration > DEBOUNCE => {
                                yield IrInterruptMessage::Pulse(duration);
                            },
                            None => {
                                last = Some(duration);
                            },
                        }
                    }
                }
            }
        }
    }

    fn normalize(input: IrInterruptMessage) -> IrInterruptMessage {
        fn round(i: u128, fac: u128) -> u128 {
            match i % fac {
                rem if rem >= fac / 2 => i + (fac - rem),
                rem => i - rem,
            }
        }
        match input {
            IrInterruptMessage::Pulse(duration) => {
                IrInterruptMessage::Pulse(Duration::from_micros(match duration.as_micros() {
                    m if m < 1000 => round(m, 10),
                    m if m < 2000 => round(m, 50),
                    m => round(m, 200),
                } as u64))
            }
            IrInterruptMessage::Timeout => IrInterruptMessage::Timeout,
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.read_stop_sender
            .send(true)
            .wrap_err("Could not send stop to ir reader")?;
        (&mut self.read_handle)
            .await
            .wrap_err("Could not wait for read thread to stop")
    }

    pub fn pulses(&self) -> Result<RwLockReadGuard<Vec<IrPulseSequence>>> {
        self.pulses
            .read()
            .map_err(|_| eyre!("Tried to acquire read lock to pulses vector"))
    }

    pub fn pulses_mut(&mut self) -> Result<RwLockWriteGuard<Vec<IrPulseSequence>>> {
        self.pulses
            .write()
            .map_err(|_| eyre!("Tried to acquire write lock to pulses vector"))
    }

    pub fn pulse_stream(&self) -> impl Stream<Item = Result<Option<IrPulseSequence>>> {
        let mut receiver = self.pulse_added_receiver.clone();
        try_stream! {
            loop {
                receiver.changed().await.wrap_err("Tried getting next pulse sequence")?;
                yield receiver.borrow().clone();
            }
        }
    }
}
