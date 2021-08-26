use itertools::Itertools;
use num_traits::AsPrimitive;
use std::convert::TryFrom;
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
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

impl AsPrimitive<usize> for IrPulse {
    fn as_(self) -> usize {
        self.0.as_()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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

pub trait IrFormat {
    const WAIT_LENGTH: u128 = 10000;
    const STD_CYCLE: u128;
    fn in_bounds(pulse: IrPulse, cycles: u128) -> bool {
        in_bounds(pulse, Self::STD_CYCLE * cycles)
    }
    fn verify_leader(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool;
    fn verify_repeat(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool;
    fn decode<T: AsRef<[IrPulse]>>(data: T) -> Result<IrPulseBytes, IrDecodeError>;
    fn encode<T: AsRef<[u8]>>(bytes: T) -> Result<IrSequence, IrEncodeError>;
}

// target

pub trait TemperatureCode: TryFrom<u32> + Into<u32>
where
    <Self as TryFrom<u32>>::Error: Display,
{
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ACMode {
    Auto,
    Warm,
    Dry,
    Cool,
    Fan,
}

impl Default for ACMode {
    fn default() -> Self {
        ACMode::Auto
    }
}

#[derive(Error, Debug)]
#[error("Invalid AC mode")]
pub struct InvalidAcMode;

impl FromStr for ACMode {
    type Err = InvalidAcMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(ACMode::Auto),
            "warm" => Ok(ACMode::Warm),
            "dry" => Ok(ACMode::Dry),
            "cool" => Ok(ACMode::Cool),
            "fan" => Ok(ACMode::Fan),
            _ => Err(InvalidAcMode),
        }
    }
}

#[derive(Debug)]
pub struct IrStatus<T: IrTarget>
where
    <<T as IrTarget>::Temperature as TryFrom<u32>>::Error: Display,
{
    pub powered: bool,
    pub mode: ACMode,
    pub temperature: T::Temperature,
}

pub trait IrTarget
where
    <<Self as IrTarget>::Temperature as TryFrom<u32>>::Error: Display,
{
    type Format: IrFormat;
    type Error: std::error::Error + Send + Sync;
    type Temperature: TemperatureCode + Send + Sync;
    const SEQ_LENGTH: usize;
    fn power_off(&mut self) -> Result<IrSequence, Self::Error>;
    fn power_on(&mut self) -> Result<IrSequence, Self::Error>;
    fn temp_up(&mut self) -> Result<IrSequence, Self::Error>;
    fn temp_down(&mut self) -> Result<IrSequence, Self::Error>;
    fn temp_set(&mut self, temp: Self::Temperature) -> Result<IrSequence, Self::Error>;
    fn mode_set(&mut self, mode: ACMode) -> Result<IrSequence, Self::Error>;
    fn status(&self) -> IrStatus<Self>
    where
        Self: Sized;
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
pub enum IrEncodeError {}

#[derive(Clone, Debug)]
pub struct IrPulseBytes(pub Vec<u8>);

impl AsRef<[u8]> for IrPulseBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub trait IrSource {
    type Format: IrFormat;
}

impl ToString for IrPulseBytes {
    fn to_string(&self) -> String {
        self.0.iter().map(|b| format!("0x{:02X}", b)).join(", ")
    }
}
