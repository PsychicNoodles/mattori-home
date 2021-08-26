pub mod types;

use thiserror::Error;

use crate::ir::format::Aeha;
use crate::ir::sanyo::types::{sanyo_sequence, SanyoTemperatureCode, SanyoTrigger};
use crate::ir::types::{ACMode, IrEncodeError, IrFormat, IrSequence, IrStatus, IrTarget};
use std::cmp::Ordering;

#[derive(Error, Clone, Debug)]
pub enum SanyoError {
    #[error("Temperature out of range")]
    TemperatureRange,
    #[error("Could not encode ir sequence")]
    EncodeError(#[from] IrEncodeError),
}

#[derive(Debug, Default)]
pub struct Sanyo {
    powered: bool,
    mode: ACMode,
    temp: SanyoTemperatureCode,
}

impl Sanyo {
    fn as_ir_sequence(
        &self,
        trigger: SanyoTrigger,
    ) -> Result<IrSequence, <Sanyo as IrTarget>::Error> {
        Ok(<Self as IrTarget>::Format::encode(sanyo_sequence(
            self.mode.clone(),
            self.temp.clone(),
            trigger,
        ))?)
    }
}

impl IrTarget for Sanyo {
    type Format = Aeha;
    type Error = SanyoError;
    type Temperature = SanyoTemperatureCode;
    const SEQ_LENGTH: usize = 136;

    fn power_off(&mut self) -> Result<IrSequence, Self::Error> {
        self.powered = false;
        self.as_ir_sequence(SanyoTrigger::Off)
    }

    fn power_on(&mut self) -> Result<IrSequence, Self::Error> {
        self.powered = true;
        self.as_ir_sequence(SanyoTrigger::On)
    }

    fn is_powered(&self) -> bool {
        self.powered
    }

    fn temp_up(&mut self) -> Result<IrSequence, Self::Error> {
        self.temp = self.temp.up().ok_or(SanyoError::TemperatureRange)?;
        self.as_ir_sequence(SanyoTrigger::Up)
    }

    fn temp_down(&mut self) -> Result<IrSequence, Self::Error> {
        self.temp = self.temp.down().ok_or(SanyoError::TemperatureRange)?;
        self.as_ir_sequence(SanyoTrigger::Down)
    }

    fn temp_set(&mut self, temp: Self::Temperature) -> Option<Result<IrSequence, Self::Error>> {
        let trigger = match self.temp.cmp(&temp) {
            Ordering::Less => SanyoTrigger::Down,
            Ordering::Equal => return None,
            Ordering::Greater => SanyoTrigger::Up,
        };
        self.temp = temp;
        Some(self.as_ir_sequence(trigger))
    }

    fn temperature(&self) -> &Self::Temperature {
        &self.temp
    }

    fn mode_set(&mut self, mode: ACMode) -> Result<IrSequence, Self::Error> {
        self.mode = mode;
        // TODO fix
        self.as_ir_sequence(SanyoTrigger::On)
    }

    fn mode(&self) -> &ACMode {
        &self.mode
    }

    fn status(&self) -> IrStatus<Self> {
        IrStatus {
            powered: self.powered,
            mode: self.mode.clone(),
            temperature: self.temp.clone(),
        }
    }
}
