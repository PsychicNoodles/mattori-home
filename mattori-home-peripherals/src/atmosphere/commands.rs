use std::thread::sleep;
use std::time::{Duration};

use num_traits::{clamp, Zero};

use crate::atmosphere::types::{AtmoI2c, Mode, Register, Result, InternalResult, AtmoI2cInternalError, BaseResult, AtmoI2cError};

impl AtmoI2c {
    pub fn verify_id(&self) -> InternalResult<bool> {
        self.read_byte(Register::ChipId)
            .map_err(AtmoI2cInternalError::ChipId)
            .map(|id| id == Self::CHIP_ID)
    }

    fn do_reset_sensor(&self) -> BaseResult<()> {
        let guard = self.lock_i2c()?;
        Self::write_register_to(&guard, Register::SoftReset, [0xb6; 32])?;
        Ok(())
    }

    pub fn reset_sensor(&self) -> InternalResult<()> {
        self.do_reset_sensor().map_err(AtmoI2cInternalError::Sensor)?;
        sleep(Duration::from_millis(4));
        Ok(())
    }

    // mutable borrow of self, so no need to maintain a mutex lock
    fn do_read_temperature(&mut self) -> InternalResult<(f32,f32)> {
        if self.mode != Mode::Normal {
            self.set_mode(Mode::Force)?;
            // todo deal with potential lock here
            self.until_status_ok()?;
        }
        let raw_temp = self
            .read24(Register::TempData)?
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

    pub fn read_temperature(&mut self) -> Result<(f32, f32)> {
        self.do_read_temperature().map_err(AtmoI2cError::Temperature)
    }

    fn do_read_pressure(&self, temp_fine: f32) -> InternalResult<f32> {
        let adc = self
            .read24(Register::PressureData)?
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
            return Err(AtmoI2cInternalError::Calculation);
        }

        let pressure = 1048576.0 - adc;
        let pressure = ((pressure - var2 / 4096.0) * 6250.0) / var1;
        let var1 = pres9 * pressure * pressure / 2147483648.0;
        let var2 = pressure * pres8 / 32768.0;
        let pressure = pressure + (var1 + var2 + pres7) / 16.0;

        Ok(pressure / 100.0)
    }

    pub fn read_pressure(&self, temp_fine: f32) -> Result<f32> {
        self.do_read_pressure(temp_fine).map_err(AtmoI2cError::Pressure)
    }

    fn do_read_humidity(&self, temp_fine: f32) -> InternalResult<f32> {
        let hum = self
            .read_register(Register::HumidData, |buf| [buf[0], buf[1]])?;
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

    pub fn read_humidity(&self, temp_fine: f32) -> Result<f32> {
        self.do_read_humidity(temp_fine).map_err(AtmoI2cError::Humidity)
    }

    pub fn read_altitude(&self, pressure: f32) -> f32 {
        44330.0 * (1.0 - (pressure / self.sea_level_pressure).powf(0.1903))
    }

    fn until_status_ok(&self) -> BaseResult<()> {
        let guard = self.lock_i2c()?;
        loop {
            if Self::status_ok(&guard)? {
                return Ok(());
            }
            sleep(Duration::from_millis(20));
        }
    }
}
