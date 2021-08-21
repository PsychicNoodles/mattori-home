use crate::ir::types::{IrDecodeError, IrEncodeError, IrFormat, IrPulse, IrPulseBytes, IrSequence};
use itertools::Itertools;
use num_traits::AsPrimitive;

pub struct Aeha {}

impl Aeha {}

impl IrFormat for Aeha {
    const STD_CYCLE: u128 = 425;
    fn verify_leader(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool {
        Self::in_bounds(*first_pulse, 8) && Self::in_bounds(*second_pulse, 4)
    }

    fn verify_repeat(first_pulse: &IrPulse, second_pulse: &IrPulse) -> bool {
        Self::in_bounds(*first_pulse, 8) && Self::in_bounds(*second_pulse, 8)
    }

    fn decode<T: AsRef<[IrPulse]>>(data: T) -> Result<IrPulseBytes, IrDecodeError> {
        struct DecodeState {
            frames: Vec<Vec<u8>>,
            byte_list: Vec<u8>,
            byte: u8,
            bit_counter: usize,
            end_of_frame: bool,
        }
        enum DecodeStep {
            Error(IrDecodeError),
            Continue(DecodeState),
            Finished(Vec<Vec<u8>>),
        }

        let data = data.as_ref();
        if data.len() < 10 {
            return Err(IrDecodeError::TooShort);
        }
        if data.len() % 2 == 0 {
            return Err(IrDecodeError::EvenInputs);
        }

        let res = data
            .iter()
            .chunks(2)
            .into_iter()
            .skip(1)
            .map(|mut chunk| (chunk.next().unwrap(), chunk.next()))
            .fold(
                DecodeStep::Continue(DecodeState {
                    frames: Vec::new(),
                    byte_list: Vec::new(),
                    byte: 0,
                    bit_counter: 0,
                    end_of_frame: false,
                }),
                |step, pulses| match step {
                    e @ DecodeStep::Error(_) => e,
                    f @ DecodeStep::Finished(_) => f,
                    DecodeStep::Continue(mut state) => {
                        if state.end_of_frame {
                            match pulses {
                                (p1, Some(p2)) => {
                                    if Self::verify_leader(p1, p2) || Self::verify_repeat(p1, p2) {
                                        DecodeStep::Continue(state)
                                    } else {
                                        DecodeStep::Error(IrDecodeError::UnknownEnd)
                                    }
                                }
                                _ => DecodeStep::Error(IrDecodeError::OddEnd),
                            }
                        } else {
                            match pulses {
                                (p1, Some(p2)) => {
                                    if Self::in_bounds(*p1, 1) {
                                        // long gap after stop means next frame
                                        if AsPrimitive::<usize>::as_(p2.0)
                                            > (<Self as IrFormat>::WAIT_LENGTH / 2) as usize
                                        {
                                            state.frames.push(state.byte_list);
                                            state.byte_list = vec![];
                                            state.byte = 0;
                                            state.end_of_frame = true;
                                            DecodeStep::Continue(state)
                                        } else if Self::in_bounds(*p2, 1) {
                                            state.bit_counter = (state.bit_counter + 1) % 8;
                                            if state.bit_counter == 0 {
                                                state.byte_list.push(state.byte);
                                                state.byte = 0;
                                            }
                                            DecodeStep::Continue(state)
                                        } else if Self::in_bounds(*p2, 3) {
                                            state.byte += 1 << state.bit_counter;
                                            state.bit_counter = (state.bit_counter + 1) % 8;
                                            if state.bit_counter == 0 {
                                                state.byte_list.push(state.byte);
                                                state.byte = 0;
                                            }
                                            DecodeStep::Continue(state)
                                        } else {
                                            DecodeStep::Error(IrDecodeError::UnknownBit)
                                        }
                                    } else {
                                        DecodeStep::Error(IrDecodeError::UnknownBit)
                                    }
                                }
                                (p1, None) => {
                                    // stop length + bit counter = byte length
                                    if Self::in_bounds(*p1, 1) && state.bit_counter == 0 {
                                        state.frames.push(state.byte_list);
                                        DecodeStep::Finished(state.frames)
                                    } else {
                                        DecodeStep::Error(IrDecodeError::InvalidBits)
                                    }
                                }
                            }
                        }
                    }
                },
            );
        match res {
            DecodeStep::Finished(r) => Ok(IrPulseBytes(r)),
            DecodeStep::Error(e) => Err(e),
            DecodeStep::Continue(_) => Err(IrDecodeError::UnexpectedEnd),
        }
    }

    fn encode<T: AsRef<[Vec<u8>]>>(bytes: T) -> Result<IrSequence, IrEncodeError> {
        bytes
            .as_ref()
            .iter()
            .fold(Ok((Vec::new(), true)), |res, bytes| match res {
                e @ Err(_) => e,
                Ok((mut code, is_first)) => {
                    if bytes.is_empty() {
                        return Err(IrEncodeError::EmptyFrame);
                    }

                    if is_first {
                        code.push(Self::WAIT_LENGTH);
                    }

                    // leader
                    code.push(Self::STD_CYCLE * 8);
                    code.push(Self::STD_CYCLE * 4);

                    // data
                    for byte in bytes {
                        let mut bits = *byte;
                        for _ in 0..8 {
                            code.push(Self::STD_CYCLE);
                            if (bits & 1) == 0 {
                                code.push(Self::STD_CYCLE);
                            } else {
                                code.push(Self::STD_CYCLE * 3);
                            }
                            bits >>= 1;
                        }
                    }

                    // stop bit
                    code.push(Self::STD_CYCLE);

                    Ok((code, false))
                }
            })
            .map(|res| IrSequence(res.0.into_iter().map(IrPulse).collect()))
    }
}
