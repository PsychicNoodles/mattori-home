use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::Formatter;
use std::time::Duration;

use itertools::Itertools;
use num_traits::{AsPrimitive, PrimInt};
use rppal::gpio::Level;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Deserialize)]
pub struct IrPulse(pub u128);

impl IrPulse {
    pub fn into_inner(self) -> u128 {
        self.0
    }
}

impl AsPrimitive<u128> for IrPulse {
    fn as_(self) -> u128 {
        self.0
    }
}

impl AsPrimitive<f64> for IrPulse {
    fn as_(self) -> f64 {
        self.0.as_()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Deserialize)]
pub struct IrSequence(pub Vec<IrPulse>);

impl IrSequence {
    pub fn into_inner(self) -> Vec<IrPulse> {
        self.0
    }
}

// target

pub trait TemperatureCode {}

pub trait ACMode {}

pub trait IrTarget {
    type Error: std::error::Error + Send + Sync;
    type Temperature: TemperatureCode;
    type Mode: ACMode;
    const SEQ_LENGTH: usize;
    fn power_off(&mut self) -> Result<IrSequence, Self::Error>;
    fn power_on(&mut self) -> Result<IrSequence, Self::Error>;
    fn temp_up(&mut self) -> Result<IrSequence, Self::Error>;
    fn temp_down(&mut self) -> Result<IrSequence, Self::Error>;
    fn temp_set(&mut self, temp: Self::Temperature) -> Result<IrSequence, Self::Error>;
    fn mode_set(&mut self, mode: Self::Mode) -> Result<IrSequence, Self::Error>;
}

// source

fn in_bounds<L: AsPrimitive<f64>, T: AsPrimitive<f64>>(length: L, target: T) -> bool {
    const TOL: f64 = 0.35;
    length.as_() > target.as_() * (1f64 - TOL) && length.as_() < target.as_() * (1f64 + TOL)
}

#[derive(Error, Debug, Clone)]
pub enum IrFormatError {
    #[error("Input is too short")]
    TooShort,
    #[error("Input has even number of items")]
    EvenInputs,
    #[error("Sequence ended with odd number of pulses")]
    OddEnd,
    #[error("Sequence was neither leader nor repeat")]
    UnknownEnd,
    #[error("Sequence ended with invalid number of bits")]
    InvalidBits,
    #[error("Unknown bit")]
    UnknownBit,
    #[error("Unexpected end of data")]
    UnexpectedEnd,
}

pub struct IrPulseBytes(Vec<Vec<u8>>);

pub trait IrFormat {
    const WAIT_LENGTH: usize = 10000;
    fn verify_leader(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool;
    fn verify_repeat(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool;
    fn decode(data: &Vec<IrPulse>) -> Result<IrPulseBytes, IrFormatError>;
}

pub struct Aeha {}

impl Aeha {
    const STD_CYCLE: usize = 425;
}

impl IrFormat for Aeha {
    fn verify_leader(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool {
        in_bounds(*first_pulse, Self::STD_CYCLE * 8)
            && in_bounds(*second_pulse, Self::STD_CYCLE * 4)
    }

    fn verify_repeat(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool {
        in_bounds(*first_pulse, Self::STD_CYCLE * 8)
            && in_bounds(*second_pulse, Self::STD_CYCLE * 8)
    }

    fn decode(data: &Vec<IrPulse>) -> Result<IrPulseBytes, IrFormatError> {
        struct DecodeState {
            frames: Vec<Vec<u8>>,
            byte_list: Vec<u8>,
            byte: u8,
            bit_counter: usize,
            end_of_frame: bool,
        }
        enum DecodeStep {
            Error(IrFormatError),
            Continue(DecodeState),
            Finished(Vec<Vec<u8>>),
        }

        if data.len() < 10 {
            return Err(IrFormatError::TooShort);
        }
        if data.len() % 2 == 0 {
            return Err(IrFormatError::EvenInputs);
        }

        let res = data
            .into_iter()
            .chunks(2)
            .into_iter()
            .skip(1)
            .map(|mut chunk| (chunk.next().unwrap(), chunk.next()))
            .fold(
                DecodeStep::Continue(DecodeState {
                    frames: Vec::new(),
                    byte_list: Vec::new(),
                    byte: 0,
                    bit_counter: 0,
                    end_of_frame: false,
                }),
                |step, pulses| match step {
                    e @ DecodeStep::Error(_) => e,
                    f @ DecodeStep::Finished(_) => f,
                    DecodeStep::Continue(mut state) => {
                        if state.end_of_frame {
                            match pulses {
                                (p1, Some(p2)) => {
                                    if Self::verify_leader(p1, p2) || Self::verify_repeat(p1, p2) {
                                        DecodeStep::Continue(state)
                                    } else {
                                        DecodeStep::Error(IrFormatError::UnknownEnd)
                                    }
                                }
                                _ => DecodeStep::Error(IrFormatError::OddEnd),
                            }
                        } else {
                            match pulses {
                                (p1, Some(p2)) => {
                                    if in_bounds(*p1, Self::STD_CYCLE) {
                                        // long gap after stop means next frame
                                        if AsPrimitive::<usize>::as_(p2.0)
                                            > <Self as IrFormat>::WAIT_LENGTH / 2
                                        {
                                            state.frames.push(state.byte_list);
                                            state.byte_list = vec![];
                                            state.byte = 0;
                                            state.end_of_frame = true;
                                            DecodeStep::Continue(state)
                                        } else if in_bounds(*p2, Self::STD_CYCLE) {
                                            state.bit_counter = (state.bit_counter + 1) % 8;
                                            if state.bit_counter == 0 {
                                                state.byte_list.push(state.byte);
                                                state.byte = 0;
                                            }
                                            DecodeStep::Continue(state)
                                        } else if in_bounds(*p2, Self::STD_CYCLE * 3) {
                                            state.byte = state.byte + (1 << state.bit_counter);
                                            state.bit_counter = (state.bit_counter + 1) % 8;
                                            if state.bit_counter == 0 {
                                                state.byte_list.push(state.byte);
                                                state.byte = 0;
                                            }
                                            DecodeStep::Continue(state)
                                        } else {
                                            DecodeStep::Error(IrFormatError::UnknownBit)
                                        }
                                    } else {
                                        DecodeStep::Error(IrFormatError::UnknownBit)
                                    }
                                }
                                (p1, None) => {
                                    // stop length + bit counter = byte length
                                    if in_bounds(*p1, Self::STD_CYCLE) && state.bit_counter == 0 {
                                        state.frames.push(state.byte_list);
                                        DecodeStep::Finished(state.frames)
                                    } else {
                                        DecodeStep::Error(IrFormatError::InvalidBits)
                                    }
                                }
                            }
                        }
                    }
                },
            );
        match res {
            DecodeStep::Finished(r) => Ok(IrPulseBytes(r)),
            DecodeStep::Error(e) => Err(e),
            DecodeStep::Continue(_) => Err(IrFormatError::UnexpectedEnd),
        }
    }
}

pub trait IrSource {
    type Format: IrFormat;
}

impl ToString for IrPulseBytes {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .map(|(i, frame)| {
                format!(
                    "Frame #{} {}",
                    i + 1,
                    if frame.is_empty() {
                        String::from("Repeat\n")
                    } else {
                        frame.iter().map(|b| format!("0x{:02X}", b)).join(", ")
                    }
                )
            })
            .join("\n")
    }
}
