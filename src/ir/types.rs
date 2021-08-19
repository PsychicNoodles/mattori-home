use std::convert::TryFrom;
use std::time::Duration;

use rppal::gpio::Level;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;
use std::cmp::Ordering;
use std::fmt::Formatter;
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IrLevel(pub Level);

struct IrLevelVisitor {}

impl IrLevelVisitor {
    fn from_lowercase_str<E: de::Error>(value: &str) -> Result<IrLevel, E> {
        match value {
            "low" => Ok(IrLevel(Level::Low)),
            "high" => Ok(IrLevel(Level::High)),
            _ => Err(E::custom(format!("invalid string: {}", value))),
        }
    }
}

impl<'de> Visitor<'de> for IrLevelVisitor {
    type Value = IrLevel;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a case-insensitive string that is either \"low\" or \"high\"")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::from_lowercase_str(v.to_lowercase().as_str())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::from_lowercase_str(v.to_lowercase().as_str())
    }
}

impl<'de> Deserialize<'de> for IrLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(IrLevelVisitor {})
    }
}

impl IrLevel {
    pub const fn into_inner(self) -> Level {
        self.0
    }
}

impl PartialOrd for IrLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IrLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.0, other.0) {
            (Level::Low, Level::Low) | (Level::High, Level::High) => Ordering::Equal,
            (Level::Low, Level::High) => Ordering::Less,
            (Level::High, Level::Low) => Ordering::Greater,
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize)]
pub struct IrPulse {
    pub level: IrLevel,
    pub duration: u64,
}

impl IrPulse {
    pub const MAX_WIDTH: u64 = 100;
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
