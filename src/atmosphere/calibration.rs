use crate::atmosphere::types::{AtmoI2c, Register};
use color_eyre::eyre::{Result, WrapErr};
use packed_struct::prelude::*;
use packed_struct::PackedStructInfo;
use rppal::i2c::I2c;
use std::convert::TryInto;
use std::sync::MutexGuard;

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct Temperature {
    pub(super) a: u16,
    pub(super) b: i16,
    pub(super) c: i16,
}

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct Pressure {
    pub(super) a: u16,
    pub(super) b: i16,
    pub(super) c: i16,
    pub(super) d: i16,
    pub(super) e: i16,
    pub(super) f: i16,
    pub(super) g: i16,
    pub(super) h: i16,
    pub(super) i: i16,
}

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct PackedHumidity {
    b: i16,
    c: u8,
    d: u8,
    e: u8,
    f: i16,
    g: i8,
}

pub struct Humidity {
    pub(super) a: u8,
    pub(super) b: i16,
    pub(super) c: u8,
    pub(super) d: i16,
    pub(super) e: i16,
    pub(super) f: i8,
}

impl Humidity {
    fn from(a: u8, PackedHumidity { b, c, d, e, f, g }: PackedHumidity) -> Humidity {
        Humidity {
            a,
            b,
            c,
            d: ((d as i16) << 4) + ((e as i16) & 0xf),
            e: ((f as i16) << 4) + ((e as i16) >> 4),
            f: g,
        }
    }
}

pub(super) struct Calibration {
    pub(super) temperature: Temperature,
    pub(super) pressure: Pressure,
    pub(super) humidity: Humidity,
}

impl AtmoI2c {
    pub(super) fn read_calibration(guard: &MutexGuard<I2c>) -> Result<Calibration> {
        let (temperature, pressure) = Self::read_register_from(
            guard,
            Register::DigT1,
            |buf| -> Result<(Temperature, Pressure)> {
                let temperature_bytes = Temperature::packed_bits() / 8;
                let pressure_bytes = Pressure::packed_bits() / 8;
                let temperature_data = &buf[..temperature_bytes]
                    .try_into()
                    .wrap_err("Calculated temperature data width mismatch")?;
                let pressure_data = &buf[temperature_bytes..(temperature_bytes + pressure_bytes)]
                    .try_into()
                    .wrap_err("Calculated pressure data width mismatch")?;
                Ok((
                    Temperature::unpack(temperature_data)
                        .wrap_err("Temperature data was of an invalid format")?,
                    Pressure::unpack(pressure_data)
                        .wrap_err("Pressure data was of an invalid format")?,
                ))
            },
        )
        .wrap_err("Could not read temperature and pressure register")?
        .wrap_err("Could not unpack temperature and pressure data")?;
        let humidity_h1 =
            Self::read_byte_from(guard, Register::DigH1).wrap_err("Could not read H1 register")?;
        let packed_humidity =
            Self::read_register_from(guard, Register::DigH2, |buf| -> Result<PackedHumidity> {
                let humidity_bytes = PackedHumidity::packed_bits() / 8;
                let humidity_data = &buf[..humidity_bytes]
                    .try_into()
                    .wrap_err("Calculated humidity data width mismatch")?;
                PackedHumidity::unpack(humidity_data)
                    .wrap_err("Humidity data was of an invalid format")
            })
            .wrap_err("Could not read humidity register")?
            .wrap_err("Could not unpack humidity data")?;
        let humidity = Humidity::from(humidity_h1, packed_humidity);
        Ok(Calibration {
            temperature,
            pressure,
            humidity,
        })
    }

    pub(crate) fn reload_calibration(&mut self) -> Result<()> {
        let calibration = Self::read_calibration(&self.lock_i2c()?)
            .wrap_err("Could not reload calibration data")?;
        self.calibration = calibration;
        Ok(())
    }
}
