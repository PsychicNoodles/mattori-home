use thiserror::Error;

use crate::ir::sanyo::types::{
    SanyoMode, SanyoTemperatureCode, SanyoTrigger, SANYO_TEMPERATURE_CODES,
};
use crate::ir::types::{IrPulse, IrSequence, IrTarget, TemperatureCode};
use core::mem;
use std::cmp::Ordering;
use std::mem::MaybeUninit;

// #[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
// pub enum SanyoTemperatureCode {
//     T16,
//     T17,
//     T18,
//     T19,
//     T20,
//     T21,
//     T22,
//     T23,
//     T24,
//     T25,
//     T26,
//     T27,
//     T28,
//     T29,
//     T30,
// }
//
// impl TemperatureCode for SanyoTemperatureCode {}
//
// impl From<SanyoTemperatureCode> for usize {
//     fn from(t: SanyoTemperatureCode) -> Self {
//         use SanyoTemperatureCode::*;
//         match t {
//             T16 => 16,
//             T17 => 17,
//             T18 => 18,
//             T19 => 19,
//             T20 => 20,
//             T21 => 21,
//             T22 => 22,
//             T23 => 23,
//             T24 => 24,
//             T25 => 25,
//             T26 => 26,
//             T27 => 27,
//             T28 => 28,
//             T29 => 29,
//             T30 => 30,
//         }
//     }
// }
//
// impl SanyoTemperatureCode {
//     pub const fn down(&self) -> Option<SanyoTemperatureCode> {
//         use SanyoTemperatureCode::*;
//         match self {
//             T16 => None,
//             T17 => Some(T16),
//             T18 => Some(T17),
//             T19 => Some(T18),
//             T20 => Some(T19),
//             T21 => Some(T20),
//             T22 => Some(T21),
//             T23 => Some(T22),
//             T24 => Some(T23),
//             T25 => Some(T24),
//             T26 => Some(T25),
//             T27 => Some(T26),
//             T28 => Some(T27),
//             T29 => Some(T28),
//             T30 => Some(T29),
//         }
//     }
//
//     pub const fn up(&self) -> Option<SanyoTemperatureCode> {
//         use SanyoTemperatureCode::*;
//         match self {
//             T16 => Some(T17),
//             T17 => Some(T18),
//             T18 => Some(T19),
//             T19 => Some(T20),
//             T20 => Some(T21),
//             T21 => Some(T22),
//             T22 => Some(T23),
//             T23 => Some(T24),
//             T24 => Some(T25),
//             T25 => Some(T26),
//             T26 => Some(T27),
//             T27 => Some(T28),
//             T28 => Some(T29),
//             T29 => Some(T30),
//             T30 => None,
//         }
//     }
//
//     pub const fn as_ir_sequences(&self, as_on: bool) -> ([IrPulse; 5], [IrPulse; 7]) {
//         use IrPulse::*;
//         match self {
//             SanyoTemperatureCode::T16 => (
//                 [Low, Low, High, High, Low],
//                 if as_on {
//                     [High, High, Low, High, High, High, Low]
//                 } else {
//                     [High, Low, Low, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T17 => (
//                 if as_on {
//                     [Low, Low, Low, Low, High]
//                 } else {
//                     [High, Low, High, High, Low]
//                 },
//                 if as_on {
//                     [High, Low, Low, Low, High, High, Low]
//                 } else {
//                     [High, High, Low, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T18 => (
//                 if as_on {
//                     [High, Low, Low, Low, High]
//                 } else {
//                     [Low, High, High, High, Low]
//                 },
//                 if as_on {
//                     [High, High, Low, Low, High, High, Low]
//                 } else {
//                     [High, Low, High, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T19 => (
//                 if as_on {
//                     [Low, High, Low, Low, High]
//                 } else {
//                     [High, High, High, High, Low]
//                 },
//                 if as_on {
//                     [High, Low, High, Low, High, High, Low]
//                 } else {
//                     [High, High, High, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T20 => (
//                 if as_on {
//                     [High, Low, High, High, Low]
//                 } else {
//                     [Low, Low, Low, Low, High]
//                 },
//                 if as_on {
//                     [Low, High, Low, High, High, High, Low]
//                 } else {
//                     [Low, High, Low, Low, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T21 => (
//                 if as_on {
//                     [Low, Low, Low, High, Low]
//                 } else {
//                     [High, Low, Low, Low, High]
//                 },
//                 if as_on {
//                     [Low, High, Low, High, High, High, Low]
//                 } else {
//                     [Low, Low, High, Low, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T22 => (
//                 if as_on {
//                     [High, Low, High, Low, High]
//                 } else {
//                     [Low, High, Low, Low, High]
//                 },
//                 if as_on {
//                     [Low, Low, High, High, High, High, Low]
//                 } else {
//                     [Low, High, High, Low, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T23 => (
//                 if as_on {
//                     [Low, High, High, Low, High]
//                 } else {
//                     [High, High, Low, Low, High]
//                 },
//                 if as_on {
//                     [Low, High, High, High, High, High, Low]
//                 } else {
//                     [Low, Low, Low, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T24 => (
//                 if as_on {
//                     [Low, High, High, Low, High]
//                 } else {
//                     [Low, Low, High, Low, High]
//                 },
//                 if as_on {
//                     [Low, Low, Low, Low, Low, Low, High]
//                 } else {
//                     [Low, High, Low, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T25 => (
//                 if as_on {
//                     [High, High, High, Low, High]
//                 } else {
//                     [High, Low, High, Low, High]
//                 },
//                 if as_on {
//                     [Low, High, Low, Low, Low, Low, High]
//                 } else {
//                     [Low, Low, High, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T26 => (
//                 if as_on {
//                     [Low, Low, Low, High, High]
//                 } else {
//                     [Low, High, High, Low, High]
//                 },
//                 if as_on {
//                     [High, Low, High, Low, High, High, Low]
//                 } else {
//                     [Low, High, High, High, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T27 => (
//                 if as_on {
//                     [High, Low, Low, High, High]
//                 } else {
//                     [High, High, High, Low, High]
//                 },
//                 if as_on {
//                     [High, Low, High, Low, High, High, Low]
//                 } else {
//                     [Low, Low, Low, Low, Low, Low, High]
//                 },
//             ),
//             SanyoTemperatureCode::T28 => (
//                 if as_on {
//                     [Low, High, Low, High, High]
//                 } else {
//                     [Low, Low, Low, High, High]
//                 },
//                 if as_on {
//                     [High, High, High, Low, High, High, Low]
//                 } else {
//                     [High, High, Low, Low, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T29 => (
//                 if as_on {
//                     [Low, High, High, High, Low]
//                 } else {
//                     [High, Low, Low, High, High]
//                 },
//                 if as_on {
//                     [Low, High, High, Low, High, High, Low]
//                 } else {
//                     [High, Low, High, Low, High, High, Low]
//                 },
//             ),
//             SanyoTemperatureCode::T30 => (
//                 if as_on {
//                     [High, High, High, High, Low]
//                 } else {
//                     [Low, High, Low, High, High]
//                 },
//                 if as_on {
//                     [Low, Low, High, High, High, High, Low]
//                 } else {
//                     [High, High, High, Low, High, High, Low]
//                 },
//             ),
//         }
//     }
//
//     pub const fn items() -> [SanyoTemperatureCode; 15] {
//         [
//             SanyoTemperatureCode::T16,
//             SanyoTemperatureCode::T17,
//             SanyoTemperatureCode::T18,
//             SanyoTemperatureCode::T19,
//             SanyoTemperatureCode::T20,
//             SanyoTemperatureCode::T21,
//             SanyoTemperatureCode::T22,
//             SanyoTemperatureCode::T23,
//             SanyoTemperatureCode::T24,
//             SanyoTemperatureCode::T25,
//             SanyoTemperatureCode::T26,
//             SanyoTemperatureCode::T27,
//             SanyoTemperatureCode::T28,
//             SanyoTemperatureCode::T29,
//             SanyoTemperatureCode::T30,
//         ]
//     }
// }
//
// impl Default for SanyoTemperatureCode {
//     fn default() -> Self {
//         SanyoTemperatureCode::T16
//     }
// }
//
// const fn sequence_head(as_on: Option<SanyoTemperatureCode>) -> [IrPulse; 49] {
//     const fn p41_on_high(code: &SanyoTemperatureCode) -> bool {
//         match *code {
//             SanyoTemperatureCode::T17
//             | SanyoTemperatureCode::T18
//             | SanyoTemperatureCode::T19
//             | SanyoTemperatureCode::T20
//             | SanyoTemperatureCode::T28
//             | SanyoTemperatureCode::T29
//             | SanyoTemperatureCode::T30 => true,
//             _ => false,
//         }
//     }
//     const fn p42_on_high(code: &SanyoTemperatureCode) -> bool {
//         match *code {
//             SanyoTemperatureCode::T16
//             | SanyoTemperatureCode::T24
//             | SanyoTemperatureCode::T25
//             | SanyoTemperatureCode::T26
//             | SanyoTemperatureCode::T27 => true,
//             _ => false,
//         }
//     }
//     [
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::High,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         match as_on.as_ref() {
//             Some(t) if p41_on_high(t) => IrPulse::High,
//             _ => IrPulse::Low,
//         },
//         match as_on.as_ref() {
//             Some(t) if p42_on_high(t) => IrPulse::High,
//             _ => IrPulse::Low,
//         },
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//     ]
// }
//
// // from 55th pulse
// const fn sequence_mid(as_on: Option<SanyoTemperatureCode>) -> [IrPulse; 74] {
//     const fn p70_on_low(code: &SanyoTemperatureCode) -> bool {
//         match *code {
//             SanyoTemperatureCode::T17
//             | SanyoTemperatureCode::T18
//             | SanyoTemperatureCode::T19
//             | SanyoTemperatureCode::T20
//             | SanyoTemperatureCode::T28
//             | SanyoTemperatureCode::T29
//             | SanyoTemperatureCode::T30 => true,
//             _ => false,
//         }
//     }
//     [
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         match as_on.as_ref() {
//             Some(t) if p70_on_low(t) => IrPulse::Low,
//             _ => IrPulse::High,
//         },
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::High,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//         IrPulse::Low,
//     ]
// }
//
// const fn sequence_tail() -> [IrPulse; 1] {
//     [IrPulse::Low]
// }

#[derive(Error, Debug)]
pub enum SanyoError {
    #[error("Temperature out of range")]
    TemperatureRange,
    #[error("Already at this temperature")]
    TemperatureSame,
    #[error("Internal error: {0}")]
    Internal(&'static str),
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
        let seqs = SANYO_TEMPERATURE_CODES
            .get(&self.mode)
            .ok_or(SanyoError::Internal("Unimplemented mode"))?
            .get(&self.temp)
            .ok_or(SanyoError::Internal("Unimplemented temperature code"))?;
        Ok(IrSequence(match trigger {
            SanyoTrigger::Up => seqs
                .up
                .as_ref()
                .ok_or(SanyoError::TemperatureRange)?
                .clone(),
            SanyoTrigger::Down => seqs
                .down
                .as_ref()
                .ok_or(SanyoError::TemperatureRange)?
                .clone(),
            SanyoTrigger::On => seqs.on.clone(),
            SanyoTrigger::Off => seqs.off.clone(),
        }))
    }
}

impl IrTarget for Sanyo {
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
