#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use rppal::gpio::Error;
use thiserror::Error;

pub mod atmosphere;
pub mod ir;
pub mod lcd;
pub mod led;

#[derive(Error, Clone, Debug)]
pub enum I2cError {
    #[error("Could not initialize i2c")]
    Initialization,
    #[error("Could not set slave address to {0}")]
    SlaveAddr(u16),
    #[error("Could not get pin {0}")]
    Pin(u8),
}

// cloneable error wrapper
#[derive(Error, Clone, Debug)]
pub enum RppalError {
    #[error("I/O error")]
    Io,
    #[error("Invalid slave address")]
    InvalidSlaveAddress(u16),
    #[error("I2C/SMBus feature not supported")]
    FeatureNotSupported,
    #[error("Unknown model")]
    UnknownModel,
    #[error("Pin is not available")]
    PinNotAvailable(u8),
    #[error("Permission denied when opening /dev/gpiomem, /dev/mem or /dev/gpiochipN for read/write access")]
    PermissionDenied(String),
    #[error("Thread panicked")]
    ThreadPanic,
}

impl From<rppal::i2c::Error> for RppalError {
    fn from(e: rppal::i2c::Error) -> Self {
        match e {
            rppal::i2c::Error::Io(_) => RppalError::Io,
            rppal::i2c::Error::InvalidSlaveAddress(a) => RppalError::InvalidSlaveAddress(a),
            rppal::i2c::Error::FeatureNotSupported => RppalError::FeatureNotSupported,
            rppal::i2c::Error::UnknownModel => RppalError::UnknownModel,
        }
    }
}

impl From<rppal::gpio::Error> for RppalError {
    fn from(e: Error) -> Self {
        match e {
            rppal::gpio::Error::UnknownModel => RppalError::UnknownModel,
            rppal::gpio::Error::PinNotAvailable(p) => RppalError::PinNotAvailable(p),
            rppal::gpio::Error::PermissionDenied(e) => RppalError::PermissionDenied(e),
            rppal::gpio::Error::Io(_) => RppalError::Io,
            rppal::gpio::Error::ThreadPanic => RppalError::ThreadPanic,
        }
    }
}
