use serde_derive::Deserialize;
use std::convert::TryFrom;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Deserialize)]
pub struct LowPulse(pub Duration);

impl Default for LowPulse {
    fn default() -> Self {
        LowPulse(Duration::from_micros(4))
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Deserialize)]
pub struct HighPulse(pub Duration);

impl Default for HighPulse {
    fn default() -> Self {
        HighPulse(Duration::from_micros(15))
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Deserialize)]
#[serde(try_from = "i64")]
pub enum IrPulse {
    Low(LowPulse),
    High(HighPulse),
}

#[derive(Error, Debug)]
#[error("Invalid ir pulse: {input}")]
pub struct InvalidIrPulse {
    input: i64,
}

impl TryFrom<i64> for IrPulse {
    type Error = InvalidIrPulse;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(IrPulse::Low(LowPulse::default())),
            1 => Ok(IrPulse::High(HighPulse::default())),
            _ => Err(InvalidIrPulse { input: value }),
        }
    }
}

impl IrPulse {
    pub const MAX_WIDTH: usize = 17;
    pub const LOW_DURATION: Duration = Duration::from_micros(4);

    pub fn is_valid_low(count: usize) -> bool {
        (3..=5).contains(&count)
    }

    pub const fn as_duration(&self) -> Duration {
        match self {
            IrPulse::Low(LowPulse(d)) | IrPulse::High(HighPulse(d)) => *d,
        }
    }
}

#[derive(Error, Debug)]
pub enum IrPulseError {
    #[error("Zero width IR pulse")]
    Zero,
    #[error("Unknown IR pulse width")]
    UnknownWidth,
    #[error("Too long IR pulse width")]
    TooLong,
}

impl TryFrom<usize> for IrPulse {
    type Error = IrPulseError;

    fn try_from(count: usize) -> Result<Self, Self::Error> {
        if (3..=7).contains(&count) {
            Ok(IrPulse::Low(LowPulse(Duration::from_micros(count as u64))))
        } else if (14..=17).contains(&count) {
            Ok(IrPulse::High(HighPulse(Duration::from_micros(
                count as u64,
            ))))
        } else if count == 0 {
            Err(IrPulseError::Zero)
        } else if count > IrPulse::MAX_WIDTH {
            Err(IrPulseError::TooLong)
        } else {
            Err(IrPulseError::UnknownWidth)
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct IrSequence(pub Vec<IrPulse>);

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
