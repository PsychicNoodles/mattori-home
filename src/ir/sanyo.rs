pub mod types;

use thiserror::Error;

use crate::ir::format::Aeha;
use crate::ir::sanyo::types::{sanyo_sequence, SanyoMode, SanyoTemperatureCode, SanyoTrigger};
use crate::ir::types::{IrEncodeError, IrFormat, IrSequence, IrTarget};
use std::cmp::Ordering;

#[derive(Error, Debug)]
pub enum SanyoError {
    #[error("Temperature out of range")]
    TemperatureRange,
    #[error("Already at this temperature")]
    TemperatureSame,
    #[error("Could not encode ir sequence")]
    EncodeError(#[from] IrEncodeError),
}

#[derive(Debug, Default)]
pub struct Sanyo {
    powered: bool,
    mode: SanyoMode,
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
    type Mode = SanyoMode;
    const SEQ_LENGTH: usize = 136;

    fn power_off(&mut self) -> Result<IrSequence, Self::Error> {
        self.as_ir_sequence(SanyoTrigger::Off)
    }

    fn power_on(&mut self) -> Result<IrSequence, Self::Error> {
        self.as_ir_sequence(SanyoTrigger::On)
    }

    fn temp_up(&mut self) -> Result<IrSequence, Self::Error> {
        self.temp = self.temp.up().ok_or(SanyoError::TemperatureRange)?;
        self.as_ir_sequence(SanyoTrigger::Up)
    }

    fn temp_down(&mut self) -> Result<IrSequence, Self::Error> {
        self.temp = self.temp.down().ok_or(SanyoError::TemperatureRange)?;
        self.as_ir_sequence(SanyoTrigger::Down)
    }

    fn temp_set(&mut self, temp: Self::Temperature) -> Result<IrSequence, Self::Error> {
        let trigger = match self.temp.cmp(&temp) {
            Ordering::Less => SanyoTrigger::Down,
            Ordering::Equal => return Err(SanyoError::TemperatureSame),
            Ordering::Greater => SanyoTrigger::Up,
        };
        self.temp = temp;
        self.as_ir_sequence(trigger)
    }

    fn mode_set(&mut self, mode: Self::Mode) -> Result<IrSequence, Self::Error> {
        self.mode = mode;
        // TODO fix
        self.as_ir_sequence(SanyoTrigger::On)
    }
}

// mod test {
//     use super::*;
//
//     #[test]
//     fn all_temperature_ir_sequences_unique_off() {
//         let mut all_sequences: Vec<_> = IntoIterator::into_iter(SanyoTemperatureCode::items())
//             .map(|c| {
//                 let mut sanyo = Sanyo::default();
//                 sanyo.temp = c;
//                 sanyo.as_ir_sequence(SanyoTrigger::Off).unwrap()
//             })
//             .collect();
//         assert_eq!(all_sequences.len(), 15);
//         all_sequences.sort_unstable();
//         all_sequences.dedup();
//         assert_eq!(all_sequences.len(), 15);
//         all_sequences
//             .into_iter()
//             .for_each(|seq| assert_eq!(seq.0.len(), Sanyo::SEQ_LENGTH));
//     }
//
//     #[test]
//     fn all_temperature_ir_sequences_unique_on() {
//         let mut all_sequences: Vec<_> = IntoIterator::into_iter(SanyoTemperatureCode::items())
//             .map(|c| {
//                 let mut sanyo = Sanyo::default();
//                 sanyo.temp = c;
//                 sanyo.as_ir_sequence(SanyoTrigger::On).unwrap()
//             })
//             .collect();
//         assert_eq!(all_sequences.len(), 15);
//         all_sequences.sort_unstable();
//         all_sequences.dedup();
//         assert_eq!(all_sequences.len(), 15);
//         all_sequences
//             .into_iter()
//             .for_each(|seq| assert_eq!(seq.0.len(), Sanyo::SEQ_LENGTH));
//     }
//
//     #[test]
//     fn all_temperature_ir_sequences_unique_down() {
//         let mut all_sequences: Vec<_> = IntoIterator::into_iter(SanyoTemperatureCode::items())
//             .map(|c| {
//                 let mut sanyo = Sanyo::default();
//                 sanyo.temp = c;
//                 sanyo.as_ir_sequence(SanyoTrigger::Down)
//             })
//             .filter(Result::is_ok)
//             .map(Result::unwrap)
//             .collect();
//         assert_eq!(all_sequences.len(), 14);
//         all_sequences.sort_unstable();
//         all_sequences.dedup();
//         assert_eq!(all_sequences.len(), 14);
//         all_sequences
//             .into_iter()
//             .for_each(|seq| assert_eq!(seq.0.len(), Sanyo::SEQ_LENGTH));
//     }
//
//     #[test]
//     fn all_temperature_ir_sequences_unique_up() {
//         let mut all_sequences: Vec<_> = IntoIterator::into_iter(SanyoTemperatureCode::items())
//             .map(|c| {
//                 let mut sanyo = Sanyo::default();
//                 sanyo.temp = c;
//                 sanyo.as_ir_sequence(SanyoTrigger::Up)
//             })
//             .filter(Result::is_ok)
//             .map(Result::unwrap)
//             .collect();
//         assert_eq!(all_sequences.len(), 14);
//         all_sequences.sort_unstable();
//         all_sequences.dedup();
//         assert_eq!(all_sequences.len(), 14);
//         all_sequences
//             .into_iter()
//             .for_each(|seq| assert_eq!(seq.0.len(), Sanyo::SEQ_LENGTH));
//     }
// }
