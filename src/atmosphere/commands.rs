use crate::atmosphere::types::{AtmoI2c, Mode, Register};
use color_eyre::eyre::{Result, WrapErr};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::time;

use rppal::i2c::I2c;
use std::convert::TryInto;
use std::sync::MutexGuard;
use std::thread::sleep;

const READ_RATE: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub(super) enum ReaderMessage {
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
    pub(super) fn read_temperature(&mut self) -> Result<(usize, f32)> {
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

        let temp_fine = (var1 + var3) as usize;
        Ok((temp_fine, temp_fine as f32 / 5120.0))
    }

    pub async fn tmp(&self, mut mode_receiver: watch::Receiver<ReaderMessage>) -> Result<()> {
        let mut last_tick = Instant::now();
        let mut next_tick = last_tick + READ_RATE;
        // loop {
        //     time::sleep_until(last_tick).await;
        //     match mode_receiver.borrow_and_update().clone() {
        //         ReaderMessage::Stop => return Ok(()),
        //     }
        // }
        todo!()
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
