use core::convert;
use std::convert::TryInto;
use std::sync::MutexGuard;

use packed_struct::prelude::*;
use packed_struct::PackedStructInfo;
use rppal::i2c::I2c;

use crate::atmosphere::types::{
    AtmoI2c, AtmoI2cBaseError, AtmoI2cError, AtmoI2cRawReadingType, BaseResult, Register, Result,
};

// bug? in packed_struct that causes an unused borrow
#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct Temperature {
    pub a: u16,
    pub b: i16,
    pub c: i16,
}

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct Pressure {
    pub a: u16,
    pub b: i16,
    pub c: i16,
    pub d: i16,
    pub e: i16,
    pub f: i16,
    pub g: i16,
    pub h: i16,
    pub i: i16,
}

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct PackedHumidity {
    b: i16,
    c: u8,
    d: i8,
    e: u8,
    f: i8,
    g: i8,
}

pub struct Humidity {
    pub a: u8,
    pub b: i16,
    pub c: u8,
    pub d: i16,
    pub e: i16,
    pub f: i8,
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

pub struct Calibration {
    pub temperature: Temperature,
    pub pressure: Pressure,
    pub humidity: Humidity,
}

impl AtmoI2c {
    pub fn read_calibration(guard: &MutexGuard<I2c>) -> Result<Calibration> {
        let (temperature, pressure) = Self::read_register_from(
            guard,
            Register::DigT1,
            |buf| -> BaseResult<(Temperature, Pressure)> {
                let temperature_bytes = Temperature::packed_bits() / 8;
                let pressure_bytes = Pressure::packed_bits() / 8;
                let temperature_data = &buf[..temperature_bytes].try_into().map_err(|_| {
                    AtmoI2cBaseError::PackedWidth(AtmoI2cRawReadingType::Temperature)
                })?;
                let pressure_data = &buf[temperature_bytes..(temperature_bytes + pressure_bytes)]
                    .try_into()
                    .map_err(|_| AtmoI2cBaseError::PackedWidth(AtmoI2cRawReadingType::Pressure))?;
                Ok((
                    Temperature::unpack(temperature_data).map_err(|source| {
                        AtmoI2cBaseError::PackedFormat(AtmoI2cRawReadingType::Temperature, source)
                    })?,
                    Pressure::unpack(pressure_data).map_err(|source| {
                        AtmoI2cBaseError::PackedFormat(AtmoI2cRawReadingType::Pressure, source)
                    })?,
                ))
            },
        )
        .and_then(convert::identity)
        .map_err(AtmoI2cError::Calibration)?;
        let humidity_h1 =
            Self::read_byte_from(guard, Register::DigH1).map_err(AtmoI2cError::Calibration)?;
        let packed_humidity = Self::read_register_from(guard, Register::DigH2, |buf| {
            let humidity_bytes = PackedHumidity::packed_bits() / 8;
            let humidity_data = &buf[..humidity_bytes]
                .try_into()
                .map_err(|_| AtmoI2cBaseError::PackedWidth(AtmoI2cRawReadingType::Humidity))?;
            PackedHumidity::unpack(humidity_data).map_err(|source| {
                AtmoI2cBaseError::PackedFormat(AtmoI2cRawReadingType::Humidity, source)
            })
        })
        .and_then(convert::identity)
        .map_err(AtmoI2cError::Calibration)?;
        let humidity = Humidity::from(humidity_h1, packed_humidity);
        Ok(Calibration {
            temperature,
            pressure,
            humidity,
        })
    }

    pub fn reload_calibration(&mut self) -> Result<()> {
        let calibration =
            Self::read_calibration(&self.lock_i2c().map_err(AtmoI2cError::Calibration)?)?;
        self.calibration = calibration;
        Ok(())
    }
}
