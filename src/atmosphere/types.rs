use std::array::IntoIter;
use std::sync::{Mutex, MutexGuard};

use color_eyre::eyre::{eyre, Result, WrapErr};
use rppal::i2c::I2c;
use tokio::time::Duration;

use crate::atmosphere::calibration::Calibration;

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub(super) enum Mode {
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
pub(super) enum Register {
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
pub(super) enum Overscan {
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

#[derive(Clone, Debug)]
pub(super) struct EnabledFeatures {
    pub(super) temperature: bool,
    pub(super) pressure: bool,
    pub(super) humidity: bool,
    pub(super) altitude: bool,
}

impl Default for EnabledFeatures {
    fn default() -> Self {
        Self {
            temperature: true,
            pressure: true,
            humidity: true,
            altitude: true,
        }
    }
}

impl EnabledFeatures {
    pub(super) fn temperature_enabled(&self) -> bool {
        self.temperature || self.pressure_enabled() || self.humidity_enabled()
    }

    pub(super) fn pressure_enabled(&self) -> bool {
        self.pressure || self.altitude_enabled()
    }

    pub(super) fn humidity_enabled(&self) -> bool {
        self.humidity
    }

    pub(super) fn altitude_enabled(&self) -> bool {
        self.altitude
    }
}

pub(super) struct AtmoI2c {
    pub(super) i2c: Mutex<I2c>,
    pub(super) mode: Mode,
    pub(super) calibration: Calibration,
    pub(super) overscan_humidity: Overscan,
    pub(super) overscan_temperature: Overscan,
    pub(super) overscan_pressure: Overscan,
    pub(super) sea_level_pressure: f32,
}

impl AtmoI2c {
    pub(super) const CHIP_ID: u8 = 0x60;
    const DEFAULT_SEA_LEVEL_PRESSURE: f32 = 1013.25;

    pub(super) fn new(addr: u16) -> Result<AtmoI2c> {
        let mut i2c = I2c::new().wrap_err("Could not initialize i2c")?;
        i2c.set_slave_address(addr)
            .wrap_err("Could not set atmosphere reader slave address")?;
        let i2c_mutex = Mutex::new(i2c);
        let calibration = Self::read_calibration(
            &i2c_mutex
                .lock()
                .map_err(|_| eyre!("Could not lock just created i2c mutex"))?,
        )
        .wrap_err("Could not read initial calibration data")?;
        let mut res = AtmoI2c {
            i2c: i2c_mutex,
            mode: Mode::Sleep,
            calibration,
            overscan_humidity: Overscan::X1,
            overscan_temperature: Overscan::X1,
            overscan_pressure: Overscan::X16,
            sea_level_pressure: Self::DEFAULT_SEA_LEVEL_PRESSURE,
        };
        res.reset_sensor().wrap_err("Could not reset sensor")?;
        res.write_ctrl_meas()
            .wrap_err("Could not write meas and hum control registers")?;
        res.write_config().wrap_err("Could not write config")?;
        if !res.verify_id()? {
            Err(eyre!("Could not find BME280"))
        } else {
            Ok(res)
        }
    }

    pub(super) fn lock_i2c(&self) -> Result<MutexGuard<I2c>> {
        self.i2c
            .lock()
            .map_err(|_| eyre!("Could not lock i2c mutex"))
    }

    pub(super) fn read_register_from<T, F: FnOnce([u8; 32]) -> T>(
        i2c_guard: &MutexGuard<I2c>,
        register: Register,
        f: F,
    ) -> Result<T> {
        let mut buf = [0u8; 32];
        i2c_guard
            .block_read(register.into(), &mut buf)
            .wrap_err_with(|| {
                format!(
                    "Could not read values {:X?} to register {}",
                    buf,
                    u8::from(register)
                )
            })
            .map(|_| buf)
            .map(f)
    }

    pub(super) fn read_register<T, F: FnOnce([u8; 32]) -> T>(
        &self,
        register: Register,
        f: F,
    ) -> Result<T> {
        Self::read_register_from(&self.lock_i2c()?, register, f)
    }

    pub(super) fn read_byte_from(guard: &MutexGuard<I2c>, register: Register) -> Result<u8> {
        Self::read_register_from(guard, register, |buf| buf[0])
    }

    pub(super) fn read_byte(&self, register: Register) -> Result<u8> {
        self.read_register(register, |buf| buf[0])
    }

    pub(super) fn read24(&self, register: Register) -> Result<f32> {
        Self::read_register_from(&self.lock_i2c()?, register, |buf| {
            IntoIter::new(buf)
                .take(3)
                .fold(0.0, |acc, b| (acc * 256.0) + (b & 0xff) as f32)
        })
    }

    pub(super) fn write_register_to(
        i2c_guard: &MutexGuard<I2c>,
        register: Register,
        mut buf: [u8; 32],
    ) -> Result<()> {
        i2c_guard
            .block_write(register.into(), &mut buf)
            .wrap_err_with(|| {
                format!(
                    "Could not write values {:X?} to register {}",
                    buf,
                    u8::from(register)
                )
            })
    }
    pub(super) fn write_register(&self, register: Register, mut buf: [u8; 32]) -> Result<()> {
        Self::write_register_to(&self.lock_i2c()?, register, buf)
    }

    pub(super) fn write_byte_to(
        guard: &MutexGuard<I2c>,
        register: Register,
        byte: u8,
    ) -> Result<()> {
        Self::write_register_to(guard, register, [byte; 32])
    }

    pub(super) fn write_byte(&self, register: Register, byte: u8) -> Result<()> {
        Self::write_byte_to(&self.lock_i2c()?, register, byte)
    }

    pub(super) fn status_ok(guard: &MutexGuard<I2c>) -> Result<bool> {
        Self::read_byte_from(guard, Register::Status).map(|status| ((status & 0x8) >> 3) != 1)
    }

    fn write_ctrl_meas(&mut self) -> Result<()> {
        self.write_byte(Register::CtrlHum, self.overscan_humidity.into())?;
        self.write_byte(
            Register::CtrlMeas,
            (u8::from(self.overscan_temperature) << 5)
                + (u8::from(self.overscan_pressure) << 2)
                + u8::from(self.mode),
        )
    }

    pub(super) fn set_mode(&mut self, mode: Mode) -> Result<()> {
        self.mode = mode;
        self.write_ctrl_meas()
    }

    pub(super) fn write_config(&mut self) -> Result<()> {
        let normal = self.mode == Mode::Normal;
        if normal {
            self.set_mode(Mode::Sleep)
                .wrap_err("Could not set mode to sleep")?;
        }
        self.write_byte(
            Register::Config,
            if self.mode == Mode::Normal {
                0x02 << 5
            } else {
                0
            },
        )
        .wrap_err("Could not write to config register")?;
        if normal {
            self.set_mode(Mode::Normal)
                .wrap_err("Could not reset mode to normal")?;
        }
        Ok(())
    }

    pub(super) fn set_sea_level_pressure(&mut self, sea_level_pressure: f32) {
        self.sea_level_pressure = sea_level_pressure;
    }
}
