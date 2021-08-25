use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use rppal::gpio::{Gpio, PwmPulse, PwmStep};
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use thiserror::Error;

use crate::ir::types::{IrSequence, IrTarget};
use core::iter;
use crate::I2cError;
use std::fmt::Debug;

const IR_OUTPUT_PIN: u8 = 13;

const WAIT_TIMEOUT: Duration = Duration::from_micros(100);

#[derive(Error, Debug)]
pub enum IrOutError<E: IrTarget + Debug> {
    #[error(transparent)]
    I2cError(#[from] I2cError),
    #[error(transparent)]
    IrTarget(E::Error),
    #[error("Could not send message to ir thread")]
    Send
}

pub type Result<T, E> = std::result::Result<T, IrOutError<E>>;

pub struct IrOut<T: 'static + IrTarget> {
    target: T,
    sequence_sender: mpsc::Sender<IrSequence>,
    send_stop_sender: watch::Sender<bool>,
}

impl<T: 'static + IrTarget + Debug> IrOut<T> {
    pub fn start(pin: u8, target: T) -> Result<IrOut<T>, T> {
        let out = Arc::new(Mutex::new(
            Gpio::new()
                .map_err(|_| I2cError::Initialization)?
                .get(pin)
                .map_err(|_| I2cError::Pin(pin))?
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

    pub fn default_pin(target: T) -> Result<Self, T> {
        Self::start(IR_OUTPUT_PIN, target)
    }

    pub fn send(&self, seq: IrSequence) -> Result<(), T> {
        debug!("sending sequence: {:?}", seq);
        self.sequence_sender
            .send(seq)
            .map_err(|_| IrOutError::Send)
    }

    pub fn stop(&mut self) -> Result<(), T> {
        self.send_stop_sender
            .send(true)
            .map_err(|_| IrOutError::Send)
    }

    pub fn send_target<F: FnMut(&mut T) -> Result<IrSequence, T>>(
        &mut self,
        mut action: F,
    ) -> Result<(), T> {
        let sequence = action(&mut self.target)?;
        self.send(sequence)
    }
}
