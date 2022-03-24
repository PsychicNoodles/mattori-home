use std::convert::TryFrom;

use cached::proc_macro::cached;
use strum_macros::EnumIter;
use thiserror::Error;
use tokio::sync::OnceCell;

use crate::ir::types::{ACMode, IrPulse, IrPulseBytes, TemperatureCode};
use core::convert;
use std::str::FromStr;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, EnumIter)]
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

impl SanyoTemperatureCode {
    pub fn ind(&self) -> u8 {
        u8::from(self) - 16
    }
}

impl Default for SanyoTemperatureCode {
    fn default() -> Self {
        SanyoTemperatureCode::T16
    }
}

impl TemperatureCode for SanyoTemperatureCode {}

#[derive(Error, Debug)]
#[error("Invalid temperature code")]
pub struct InvalidSanyoTemperatureCode;

impl TryFrom<u32> for SanyoTemperatureCode {
    type Error = InvalidSanyoTemperatureCode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        use SanyoTemperatureCode::*;
        match value {
            16 => Ok(T16),
            17 => Ok(T17),
            18 => Ok(T18),
            19 => Ok(T19),
            20 => Ok(T20),
            21 => Ok(T21),
            22 => Ok(T22),
            23 => Ok(T23),
            24 => Ok(T24),
            25 => Ok(T25),
            26 => Ok(T26),
            27 => Ok(T27),
            28 => Ok(T28),
            29 => Ok(T29),
            30 => Ok(T30),
            _ => Err(InvalidSanyoTemperatureCode),
        }
    }
}

impl FromStr for SanyoTemperatureCode {
    type Err = InvalidSanyoTemperatureCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_lowercase()
            .parse::<u32>()
            .map_err(|_| InvalidSanyoTemperatureCode)
            .map(SanyoTemperatureCode::try_from)
            .and_then(convert::identity)
    }
}

impl From<&SanyoTemperatureCode> for u8 {
    fn from(code: &SanyoTemperatureCode) -> Self {
        match code {
            SanyoTemperatureCode::T16 => 16,
            SanyoTemperatureCode::T17 => 17,
            SanyoTemperatureCode::T18 => 18,
            SanyoTemperatureCode::T19 => 19,
            SanyoTemperatureCode::T20 => 20,
            SanyoTemperatureCode::T21 => 21,
            SanyoTemperatureCode::T22 => 22,
            SanyoTemperatureCode::T23 => 23,
            SanyoTemperatureCode::T24 => 24,
            SanyoTemperatureCode::T25 => 25,
            SanyoTemperatureCode::T26 => 26,
            SanyoTemperatureCode::T27 => 27,
            SanyoTemperatureCode::T28 => 28,
            SanyoTemperatureCode::T29 => 29,
            SanyoTemperatureCode::T30 => 30,
        }
    }
}

impl From<SanyoTemperatureCode> for u32 {
    fn from(code: SanyoTemperatureCode) -> Self {
        u8::from(&code) as u32
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

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum SanyoTrigger {
    Up,
    Down,
    On,
    Off,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct SanyoTemperatureCodeSequence {
    pub up: Option<Vec<IrPulse>>,
    pub down: Option<Vec<IrPulse>>,
    pub on: Vec<IrPulse>,
    pub off: Vec<IrPulse>,
}

lazy_static! {
    static ref BASE_SEQUENCE: [OnceCell<u8>; 17] = [
        OnceCell::new_with(Some(64)),
        OnceCell::new_with(Some(0)),
        OnceCell::new_with(Some(20)),
        OnceCell::new_with(Some(128)),
        OnceCell::new_with(Some(67)),
        OnceCell::new(),
        OnceCell::new(),
        OnceCell::new_with(Some(64)),
        OnceCell::new(),
        OnceCell::new_with(Some(0)),
        OnceCell::new_with(Some(104)),
        OnceCell::new_with(Some(0)),
        OnceCell::new_with(Some(0)),
        OnceCell::new_with(Some(1)),
        OnceCell::new_with(Some(0)),
        OnceCell::new_with(Some(0)),
        OnceCell::new(),
    ];
}

fn build_sequence(byte5: u8, byte6: u8, byte8: u8, byte16: u8) -> Vec<u8> {
    let seq = BASE_SEQUENCE.clone();
    seq[5]
        .set(byte5)
        .expect("Whoopsie setting up Sanyo sequence! Tried setting an already set byte!");
    seq[6]
        .set(byte6)
        .expect("Whoopsie setting up Sanyo sequence! Tried setting an already set byte!");
    seq[8]
        .set(byte8)
        .expect("Whoopsie setting up Sanyo sequence! Tried setting an already set byte!");
    seq[16]
        .set(byte16)
        .expect("Whoopsie setting up Sanyo sequence! Tried setting an already set byte!");
    IntoIterator::into_iter(seq)
        .map(|oc| {
            oc.into_inner()
                .expect("Whoospie setting up Sanyo sequence! Not all the bytes were set!")
        })
        .collect()
}

#[cached]
pub fn sanyo_sequence(
    mode: ACMode,
    temperature: SanyoTemperatureCode,
    trigger: SanyoTrigger,
) -> IrPulseBytes {
    // todo determine how mode affects values
    let _ = match mode {
        ACMode::Cool => (),
        _ => (),
    };
    IrPulseBytes(build_sequence(
        match trigger {
            SanyoTrigger::Down | SanyoTrigger::Up => 132,
            SanyoTrigger::Off => 133,
            SanyoTrigger::On => 134,
        },
        24 + (temperature.ind() * 2),
        match trigger {
            SanyoTrigger::Off => 3,
            SanyoTrigger::Down | SanyoTrigger::On | SanyoTrigger::Up => 35,
        },
        match temperature {
            SanyoTemperatureCode::T16
            | SanyoTemperatureCode::T17
            | SanyoTemperatureCode::T18
            | SanyoTemperatureCode::T19 => 60 + (temperature.ind() * 2),
            SanyoTemperatureCode::T28 | SanyoTemperatureCode::T29 | SanyoTemperatureCode::T30 => {
                54 + ((temperature.ind() - 12) * 2)
            }
            _ => 53 + ((temperature.ind() - 4) * 2),
        } + match trigger {
            SanyoTrigger::Down | SanyoTrigger::Up => 1,
            SanyoTrigger::Off => 0,
            SanyoTrigger::On => 3,
        },
    ))
}
