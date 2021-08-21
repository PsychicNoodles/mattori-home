use crate::ir::input::IrPulseSequence;
use itertools::Itertools;
use num_traits::AsPrimitive;
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

impl AsRef<[IrPulse]> for IrSequence {
    fn as_ref(&self) -> &[IrPulse] {
        &self.0
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
pub enum IrDecodeError {
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

#[derive(Error, Debug, Clone)]
pub enum IrEncodeError {
    #[error("A frame was empty")]
    EmptyFrame,
}

pub struct IrPulseBytes(pub Vec<Vec<u8>>);

impl AsRef<[Vec<u8>]> for IrPulseBytes {
    fn as_ref(&self) -> &[Vec<u8>] {
        &self.0
    }
}

pub trait IrFormat {
    const WAIT_LENGTH: u128 = 10000;
    const STD_CYCLE: u128;
    fn in_bounds(pulse: IrPulse, cycles: u128) -> bool {
        in_bounds(pulse, Self::STD_CYCLE * cycles)
    }
    fn verify_leader(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool;
    fn verify_repeat(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool;
    fn decode<T: AsRef<[IrPulse]>>(data: T) -> Result<IrPulseBytes, IrDecodeError>;
    fn encode<T: AsRef<[Vec<u8>]>>(bytes: T) -> Result<IrSequence, IrEncodeError>;
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
