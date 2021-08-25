#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use thiserror::Error;

pub mod atmosphere;
pub mod ir;
pub mod lcd;
pub mod led;

#[derive(Error, Debug)]
pub enum I2cError {
    #[error("Could not initialize i2c")]
    Initialization,
    #[error("Could not set slave address to {0}")]
    SlaveAddr(u16),
    #[error("Could not get pin {0}")]
    Pin(u8)
}