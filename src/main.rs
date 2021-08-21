extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use crate::ir::format::Aeha;
use crate::ir::output::IrOut;
use crate::ir::sanyo::types::SanyoTemperatureCode;
use crate::ir::sanyo::Sanyo;
use crate::ir::types::{IrFormat, IrSequence, IrTarget};
use futures::{pin_mut, StreamExt};
use ir::input::IrIn;
use lcd::Lcd;
use rppal::gpio::{Gpio, Level};
use rppal::i2c::I2c;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::thread::sleep;
use std::time::Duration;

mod ir;
mod lcd;

const YELLOW_LED_PIN: u8 = 5;
const GREEN_LED_PIN: u8 = 6;

const IR_INPUT_PIN: u8 = 4;
const IR_OUTPUT_PIN: u8 = 13;

const LCD_SLAVE_ADDR: u16 = 0x3e;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // let mut lcd = Lcd::new(LCD_SLAVE_ADDR)?;
    // lcd.push_str("this is a long string!")?;
    // lcd.wait_for_processing().await?;
    // sleep(Duration::from_secs(3));
    // lcd.shutdown().await?;

    // let mut ir = IrIn::start(IR_INPUT_PIN)?;
    // let ir_stream = ir.pulse_stream();
    // pin_mut!(ir_stream);
    // let pulse_seq = ir_stream.next().await.unwrap().unwrap().unwrap();
    // ir.stop().await?;
    // println!(
    //     "pulse seq: {:?}",
    //     pulse_seq.iter().map(|p| p.into_inner()).collect::<Vec<_>>()
    // );
    // println!("{}", Aeha::decode(pulse_seq.deref())?.to_string());
    // sleep(Duration::from_secs(3));
    let mut out = IrOut::start(IR_OUTPUT_PIN, Sanyo::default())?;
    // let vec = (*pulse_seq).clone();
    // out.send(IrSequence(vec))?;
    // sleep(Duration::from_secs(2));

    // let name = if let Some(n) = std::env::args().nth(1) {
    //     n
    // } else {
    //     String::from("ir-input")
    // };
    //
    // match ir_stream.next().await {
    //     Some(Ok(Some(sequence))) => {
    //         println!("pulse seq: {:?}", sequence.0);
    //
    //         let mut f = File::create(name)?;
    //         let levels = Aeha::decode(&sequence.0)?
    //             .0
    //             .iter()
    //             .map(|l| format!("{:?}", l))
    //             .collect::<Vec<_>>();
    //         f.write_all(levels.join("\n").as_bytes())?;
    //     }
    //     _ => {
    //         eprintln!("error reading pulse sequence!");
    //     }
    // }
    // ir.stop().await?;

    println!("starting 26 deg cool");
    out.send_target(|t| {
        t.temp_set(SanyoTemperatureCode::T26)?;
        t.power_on()
    })?;
    sleep(Duration::from_secs(3));
    println!("turning off");
    out.send_target(IrTarget::power_off)?;

    // let mut yellow_pin = Gpio::new()?.get(YELLOW_LED_PIN)?.into_output();
    // let mut green_pin = Gpio::new()?.get(GREEN_LED_PIN)?.into_output();

    Ok(())
}
