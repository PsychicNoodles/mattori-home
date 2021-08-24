use std::convert::TryInto;
use std::sync::{mpsc, MutexGuard};
use std::thread::sleep;

use color_eyre::eyre::{eyre, Result, WrapErr};
use futures::Stream;
use num_traits::{clamp, Zero};
use rppal::i2c::I2c;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::time;

use crate::atmosphere::types::{AtmoI2c, EnabledFeatures, Mode, Register};
use crate::atmosphere::Reading;
use tokio::task::spawn_blocking;

#[derive(Clone, Debug)]
pub(super) enum ReaderMessage {
    Start,
    Pause,
    ChangeEnabled(EnabledFeatures),
    Stop,
}

impl AtmoI2c {
    pub(super) fn verify_id(&self) -> Result<bool> {
        self.read_byte(Register::ChipId)
            .wrap_err("Could not read chip id register")
            .map(|id| id == Self::CHIP_ID)
    }

    pub(super) fn reset_sensor(&self) -> Result<()> {
        let guard = self.lock_i2c()?;
        Self::write_register_to(&guard, Register::SoftReset, [0xb6; 32])?;
        sleep(Duration::from_millis(4));
        Ok(())
    }

    // mutable borrow of self, so no need to maintain a mutex lock
    pub(super) fn read_temperature(&mut self) -> Result<(f32, f32)> {
        if self.mode != Mode::Normal {
            self.set_mode(Mode::Force)
                .wrap_err("Could not set mode to force")?;
            self.until_status_ok()
                .wrap_err("Could not check if status was ok")?;
        }
        let raw_temp = self
            .read24(Register::TempData)
            .wrap_err("Could not read from tempdata register")?
            / 16.0;
        let temperature = &self.calibration.temperature;
        let (temp1, temp2, temp3) = (
            temperature.a as f32,
            temperature.b as f32,
            temperature.c as f32,
        );
        let var1 = ((raw_temp / 16384.0) - (temp1 / 1024.0)) * temp2;
        let var2 = raw_temp / 131072.0 - temp1 / 8192.0;
        let var3 = (var2 * var2) * temp3;

        let temp_fine = (var1 + var3).floor();
        Ok((temp_fine, temp_fine / 5120.0))
    }

    pub(super) fn read_pressure(&self, temp_fine: f32) -> Result<f32> {
        let adc = self
            .read24(Register::PressureData)
            .wrap_err("Could not read pressure data register")?
            / 16.0;
        let pressure = &self.calibration.pressure;
        let (pres1, pres2, pres3, pres4, pres5, pres6, pres7, pres8, pres9) = (
            pressure.a as f32,
            pressure.b as f32,
            pressure.c as f32,
            pressure.d as f32,
            pressure.e as f32,
            pressure.f as f32,
            pressure.g as f32,
            pressure.h as f32,
            pressure.i as f32,
        );
        let var1 = temp_fine / 2.0 - 64000.0;
        let var2 = var1 * var1 * pres6 / 32768.0;
        let var2 = var2 + var1 * pres5 * 2.0;
        let var2 = var2 / 4.0 + pres4 * 65536.0;
        let var3 = pres3 * var1 * var1 / 524288.0;
        let var1 = (var3 + pres2 * var1) / 524288.0;
        let var1 = (1.0 + var1 / 32768.0) * pres1;

        if var1.is_zero() {
            return Err(eyre!(
                "Invalid result calculating pressure from calibration registers"
            ));
        }

        let pressure = 1048576.0 - adc;
        let pressure = ((pressure - var2 / 4096.0) * 6250.0) / var1;
        let var1 = pres9 * pressure * pressure / 2147483648.0;
        let var2 = pressure * pres8 / 32768.0;
        let pressure = pressure + (var1 + var2 + pres7) / 16.0;

        Ok(pressure / 100.0)
    }

    pub(super) fn read_humidity(&self, temp_fine: f32) -> Result<f32> {
        let hum = self
            .read_register(Register::HumidData, |buf| [buf[0], buf[1]])
            .wrap_err("Could not read humidity data register")?;
        let adc = ((hum[0] as i32) << 8 | hum[1] as i32) as f32;
        let humidity = &self.calibration.humidity;
        let (hum1, hum2, hum3, hum4, hum5, hum6) = (
            humidity.a as f32,
            humidity.b as f32,
            humidity.c as f32,
            humidity.d as f32,
            humidity.e as f32,
            humidity.f as f32,
        );
        let var1 = temp_fine - 76800.0;
        let var2 = hum4 * 64.0 + (hum5 / 16384.0) * var1;
        let var3 = adc - var2;
        let var4 = hum2 / 65536.0;
        let var5 = 1.0 + (hum3 / 67108864.0) * var1;
        let var6 = 1.0 + (hum6 / 67108864.0) * var1 * var5;
        let var6 = var3 * var4 * (var5 * var6);
        let humidity = var6 * (1.0 - hum1 * var6 / 524288.0);

        Ok(clamp(humidity, 0.0, 100.0))
    }

    pub(super) fn read_altitude(&self, pressure: f32) -> f32 {
        44330.0 * (1.0 - (pressure / self.sea_level_pressure).powf(0.1903))
    }

    fn until_status_ok(&self) -> Result<()> {
        let guard = self.lock_i2c()?;
        loop {
            if Self::status_ok(&guard)? {
                return Ok(());
            }
            sleep(Duration::from_millis(20));
        }
    }
}
