use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use rppal::gpio::{Gpio, OutputPin};
use std::convert::TryFrom;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug)]
pub enum Leds {
    Green,
    Yellow,
}

#[derive(Error, Debug)]
#[error("Invalid led name")]
pub struct ParseLedsError {}

impl FromStr for Leds {
    type Err = ParseLedsError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "green" => Ok(Leds::Green),
            "yellow" => Ok(Leds::Yellow),
            _ => Err(ParseLedsError {}),
        }
    }
}

impl From<Leds> for u8 {
    fn from(l: Leds) -> Self {
        match l {
            Leds::Green => 6,
            Leds::Yellow => 5,
        }
    }
}

pub struct Led {
    pin: OutputPin,
}

impl Led {
    pub fn new(pin: u8) -> Result<Led> {
        let led = Gpio::new()
            .wrap_err("Could not initialize gpio")?
            .get(pin)
            .wrap_err_with(|| format!("Could not get gpio pin {}", pin))?
            .into_output();
        Ok(Led { pin: led })
    }

    pub fn from_led(led: Leds) -> Result<Led> {
        Self::new(u8::from(led))
    }

    pub fn on(&mut self) {
        self.pin.set_high();
    }

    pub fn off(&mut self) {
        self.pin.set_low();
    }

    pub fn is_on(&self) -> bool {
        self.pin.is_set_high()
    }
}
