use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::Duration;

use color_eyre::eyre::{eyre, Result, WrapErr};
use serde_derive::Deserialize;
use thiserror::Error;
use toml::Value;

use crate::ir::types::{ACMode, IrPulse, TemperatureCode};

const TEMPERATURE_CODE_SEQUENCES_RAW: &str = include_str!("temperature_code_sequences.toml");

#[derive(Deserialize, Debug, Hash, Eq, PartialEq)]
#[serde(try_from = "String")]
pub enum SanyoMode {
    Cool,
}

impl Default for SanyoMode {
    fn default() -> Self {
        SanyoMode::Cool
    }
}

impl ACMode for SanyoMode {}

#[derive(Error, Debug)]
#[error("Invalid mode: {input}")]
pub struct InvalidSanyoMode {
    input: String,
}

impl TryFrom<String> for SanyoMode {
    type Error = InvalidSanyoMode;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "cool" => Ok(SanyoMode::Cool),
            _ => Err(InvalidSanyoMode { input: value }),
        }
    }
}

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(try_from = "String")]
pub enum SanyoTemperatureCode {
    T16,
    T17,
    T18,
    T19,
    T20,
    T21,
    T22,
    T23,
    T24,
    T25,
    T26,
    T27,
    T28,
    T29,
    T30,
}

impl Default for SanyoTemperatureCode {
    fn default() -> Self {
        SanyoTemperatureCode::T16
    }
}

impl TemperatureCode for SanyoTemperatureCode {}

#[derive(Error, Debug)]
#[error("Invalid termpature code: {input}")]
pub struct InvalidSanyoTemperatureCode {
    input: String,
}

impl TryFrom<String> for SanyoTemperatureCode {
    type Error = InvalidSanyoTemperatureCode;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        use SanyoTemperatureCode::*;
        match value.to_lowercase().as_str() {
            "16" => Ok(T16),
            "17" => Ok(T17),
            "18" => Ok(T18),
            "19" => Ok(T19),
            "20" => Ok(T20),
            "21" => Ok(T21),
            "22" => Ok(T22),
            "23" => Ok(T23),
            "24" => Ok(T24),
            "25" => Ok(T25),
            "26" => Ok(T26),
            "27" => Ok(T27),
            "28" => Ok(T28),
            "29" => Ok(T29),
            "30" => Ok(T30),
            _ => Err(InvalidSanyoTemperatureCode { input: value }),
        }
    }
}

impl SanyoTemperatureCode {
    pub const fn down(&self) -> Option<SanyoTemperatureCode> {
        use SanyoTemperatureCode::*;
        match self {
            T16 => None,
            T17 => Some(T16),
            T18 => Some(T17),
            T19 => Some(T18),
            T20 => Some(T19),
            T21 => Some(T20),
            T22 => Some(T21),
            T23 => Some(T22),
            T24 => Some(T23),
            T25 => Some(T24),
            T26 => Some(T25),
            T27 => Some(T26),
            T28 => Some(T27),
            T29 => Some(T28),
            T30 => Some(T29),
        }
    }

    pub const fn up(&self) -> Option<SanyoTemperatureCode> {
        use SanyoTemperatureCode::*;
        match self {
            T16 => Some(T17),
            T17 => Some(T18),
            T18 => Some(T19),
            T19 => Some(T20),
            T20 => Some(T21),
            T21 => Some(T22),
            T22 => Some(T23),
            T23 => Some(T24),
            T24 => Some(T25),
            T25 => Some(T26),
            T26 => Some(T27),
            T27 => Some(T28),
            T28 => Some(T29),
            T29 => Some(T30),
            T30 => None,
        }
    }

    pub const fn items() -> [SanyoTemperatureCode; 15] {
        [
            SanyoTemperatureCode::T16,
            SanyoTemperatureCode::T17,
            SanyoTemperatureCode::T18,
            SanyoTemperatureCode::T19,
            SanyoTemperatureCode::T20,
            SanyoTemperatureCode::T21,
            SanyoTemperatureCode::T22,
            SanyoTemperatureCode::T23,
            SanyoTemperatureCode::T24,
            SanyoTemperatureCode::T25,
            SanyoTemperatureCode::T26,
            SanyoTemperatureCode::T27,
            SanyoTemperatureCode::T28,
            SanyoTemperatureCode::T29,
            SanyoTemperatureCode::T30,
        ]
    }
}

pub enum SanyoTrigger {
    Up,
    Down,
    On,
    Off,
}

#[derive(Deserialize)]
pub struct SanyoTemperatureCodeSequence {
    pub up: Option<Vec<IrPulse>>,
    pub down: Option<Vec<IrPulse>>,
    pub on: Vec<IrPulse>,
    pub off: Vec<IrPulse>,
}

lazy_static! {
    pub static ref SANYO_TEMPERATURE_CODES: HashMap<SanyoMode, HashMap<SanyoTemperatureCode, SanyoTemperatureCodeSequence>> = {
        toml::from_str(TEMPERATURE_CODE_SEQUENCES_RAW)
            .expect("Could not parse Sanyo temperature code sequences")
    };
}
