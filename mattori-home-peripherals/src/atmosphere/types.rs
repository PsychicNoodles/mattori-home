use std::sync::{Mutex, MutexGuard};

use rppal::i2c::I2c;
use thiserror::Error;

use crate::atmosphere::calibration::Calibration;
use crate::{I2cError, RppalError};

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub enum Mode {
    Sleep,
    Force,
    Normal,
}

impl From<Mode> for u8 {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Sleep => 0x00,
            Mode::Force => 0x01,
            Mode::Normal => 0x03,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Register {
    ChipId,
    Status,
    SoftReset,
    CtrlHum,
    CtrlMeas,
    Config,
    DigT1,
    DigH1,
    DigH2,
    TempData,
    PressureData,
    HumidData,
}

impl From<Register> for u8 {
    fn from(r: Register) -> Self {
        match r {
            Register::ChipId => 0xd0,
            Register::Status => 0xf3,
            Register::SoftReset => 0xe0,
            Register::CtrlHum => 0xf2,
            Register::CtrlMeas => 0xf4,
            Register::Config => 0xf5,
            Register::DigT1 => 0x88,
            Register::DigH1 => 0xa1,
            Register::DigH2 => 0xe1,
            Register::TempData => 0xfa,
            Register::PressureData => 0xf7,
            Register::HumidData => 0xfd,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Overscan {
    X1,
    X16,
}

impl From<Overscan> for u8 {
    fn from(o: Overscan) -> Self {
        match o {
            Overscan::X1 => 0x01,
            Overscan::X16 => 0x05,
        }
    }
}

pub struct AtmoI2c {
    pub i2c: Mutex<I2c>,
    pub mode: Mode,
    pub calibration: Calibration,
    pub overscan_humidity: Overscan,
    pub overscan_temperature: Overscan,
    pub overscan_pressure: Overscan,
    pub sea_level_pressure: f32,
}

#[derive(Debug, Clone)]
pub enum AtmoI2cRawReadingType {
    Temperature,
    Pressure,
    Humidity,
}

/// Low level errors
#[derive(Error, Clone, Debug)]
pub enum AtmoI2cBaseError {
    #[error("Could not read from register {0:?}")]
    ReadRegister(Register, #[source] RppalError),
    #[error("Could not write to register {0:?}")]
    WriteRegister(Register, #[source] RppalError),
    #[error("Could not acquire i2c mutex")]
    Mutex,
    #[error("Packed data was wrong width")]
    PackedWidth(AtmoI2cRawReadingType),
    #[error("Packed data was of invalid format")]
    PackedFormat(AtmoI2cRawReadingType, #[source] packed_struct::PackingError),
}

/// Errors arising from internal mechanisms
#[derive(Error, Clone, Debug)]
pub enum AtmoI2cInternalError {
    #[error("Could not write to control measure register")]
    ControlMeasure(#[source] AtmoI2cBaseError),
    #[error("Could not set mode")]
    Mode(#[source] AtmoI2cBaseError),
    #[error("Could not reset sensor")]
    Sensor(#[source] AtmoI2cBaseError),
    #[error("Could not verify chip id")]
    ChipId(#[source] AtmoI2cBaseError),
    #[error("Invalid calculating result")]
    Calculation,
    #[error(transparent)]
    BaseError(#[from] AtmoI2cBaseError),
}

/// Higher level action errors
#[derive(Error, Clone, Debug)]
pub enum AtmoI2cError {
    #[error(transparent)]
    I2c(#[from] I2cError),
    #[error("Could not read calibration data")]
    Calibration(#[source] AtmoI2cBaseError),
    #[error("Could not write to config register")]
    Config(#[source] AtmoI2cInternalError),
    #[error("Could not find BME280")]
    Unverified,
    #[error("Could not read temperature")]
    Temperature(#[source] AtmoI2cInternalError),
    #[error("Could not read pressure")]
    Pressure(#[source] AtmoI2cInternalError),
    #[error("Could not read humidity")]
    Humidity(#[source] AtmoI2cInternalError),
    #[error(transparent)]
    Internal(#[from] AtmoI2cInternalError),
}

pub type Result<T> = std::result::Result<T, AtmoI2cError>;
pub type InternalResult<T> = std::result::Result<T, AtmoI2cInternalError>;
pub type BaseResult<T> = std::result::Result<T, AtmoI2cBaseError>;

impl AtmoI2c {
    pub const CHIP_ID: u8 = 0x60;
    const DEFAULT_SEA_LEVEL_PRESSURE: f32 = 1013.25;

    pub fn new(addr: u16) -> Result<AtmoI2c> {
        let mut i2c = I2c::new().map_err(|_| I2cError::Initialization)?;
        i2c.set_slave_address(addr)
            .map_err(|_| I2cError::SlaveAddr(addr))?;
        let i2c_mutex = Mutex::new(i2c);
        let calibration = Self::read_calibration(
            &i2c_mutex
                .lock()
                .map_err(|_| AtmoI2cBaseError::Mutex)
                .map_err(AtmoI2cInternalError::BaseError)?,
        )?;
        let mut res = AtmoI2c {
            i2c: i2c_mutex,
            mode: Mode::Sleep,
            calibration,
            overscan_humidity: Overscan::X1,
            overscan_temperature: Overscan::X1,
            overscan_pressure: Overscan::X16,
            sea_level_pressure: Self::DEFAULT_SEA_LEVEL_PRESSURE,
        };
        res.reset_sensor()?;
        res.write_ctrl_meas()?;
        res.write_config()?;
        if !res.verify_id()? {
            Err(AtmoI2cError::Unverified)
        } else {
            Ok(res)
        }
    }

    pub fn lock_i2c(&self) -> BaseResult<MutexGuard<I2c>> {
        self.i2c.lock().map_err(|_| AtmoI2cBaseError::Mutex)
    }

    pub fn read_register_from<T, F: FnOnce([u8; 32]) -> T>(
        i2c_guard: &MutexGuard<I2c>,
        register: Register,
        f: F,
    ) -> BaseResult<T> {
        let mut buf = [0u8; 32];
        i2c_guard
            .block_read(register.into(), &mut buf)
            .map_err(|source| AtmoI2cBaseError::ReadRegister(register, RppalError::from(source)))
            .map(|_| buf)
            .map(f)
    }

    pub fn read_register<T, F: FnOnce([u8; 32]) -> T>(
        &self,
        register: Register,
        f: F,
    ) -> BaseResult<T> {
        Self::read_register_from(&self.lock_i2c()?, register, f)
    }

    pub fn read_byte_from(guard: &MutexGuard<I2c>, register: Register) -> BaseResult<u8> {
        Self::read_register_from(guard, register, |buf| buf[0])
    }

    pub fn read_byte(&self, register: Register) -> BaseResult<u8> {
        self.read_register(register, |buf| buf[0])
    }

    pub fn read24(&self, register: Register) -> BaseResult<f32> {
        Self::read_register_from(&self.lock_i2c()?, register, |buf| {
            IntoIterator::into_iter(buf)
                .take(3)
                .fold(0.0, |acc, b| (acc * 256.0) + b as f32)
        })
    }

    pub fn write_register_to(
        i2c_guard: &MutexGuard<I2c>,
        register: Register,
        buf: [u8; 32],
    ) -> BaseResult<()> {
        i2c_guard
            .block_write(register.into(), &buf)
            .map_err(|source| AtmoI2cBaseError::WriteRegister(register, RppalError::from(source)))
    }

    // pub fn write_register(&self, register: Register, buf: [u8; 32]) -> Result<()> {
    //     Self::write_register_to(&self.lock_i2c()?, register, buf)
    // }

    pub fn write_byte_to(guard: &MutexGuard<I2c>, register: Register, byte: u8) -> BaseResult<()> {
        Self::write_register_to(guard, register, [byte; 32])
    }

    pub fn write_byte(&self, register: Register, byte: u8) -> BaseResult<()> {
        Self::write_byte_to(&self.lock_i2c()?, register, byte)
    }

    pub fn status_ok(guard: &MutexGuard<I2c>) -> BaseResult<bool> {
        Self::read_byte_from(guard, Register::Status).map(|status| ((status & 0x8) >> 3) != 1)
    }

    fn write_ctrl_meas(&mut self) -> InternalResult<()> {
        self.write_byte(Register::CtrlHum, self.overscan_humidity.into())
            .map_err(AtmoI2cInternalError::ControlMeasure)?;
        self.write_byte(
            Register::CtrlMeas,
            (u8::from(self.overscan_temperature) << 5)
                + (u8::from(self.overscan_pressure) << 2)
                + u8::from(self.mode),
        )
        .map_err(AtmoI2cInternalError::ControlMeasure)?;
        Ok(())
    }

    pub fn set_mode(&mut self, mode: Mode) -> InternalResult<()> {
        self.mode = mode;
        self.write_ctrl_meas().map_err(|e| match e {
            AtmoI2cInternalError::ControlMeasure(e) => AtmoI2cInternalError::Mode(e),
            _ => panic!("should always be a control measure error"),
        })
    }

    pub fn write_config(&mut self) -> Result<()> {
        let normal = self.mode == Mode::Normal;
        if normal {
            self.set_mode(Mode::Sleep).map_err(AtmoI2cError::Config)?;
        }
        self.write_byte(
            Register::Config,
            if self.mode == Mode::Normal {
                0x02 << 5
            } else {
                0
            },
        )
        .map_err(AtmoI2cInternalError::BaseError)
        .map_err(AtmoI2cError::Config)?;
        if normal {
            self.set_mode(Mode::Normal).map_err(AtmoI2cError::Config)?;
        }
        Ok(())
    }

    pub fn set_sea_level_pressure(&mut self, sea_level_pressure: f32) {
        self.sea_level_pressure = sea_level_pressure;
    }
}
