extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use crate::atmosphere::Atmosphere;
use crate::ir::output::IrOut;
use crate::ir::sanyo::Sanyo;
use crate::ir::types::{IrFormat, IrPulse, IrSequence, IrTarget};
use crate::lcd::Lcd;
use crate::led::{Led, Leds};
use color_eyre::eyre::WrapErr;
use futures::{pin_mut, StreamExt};
use ir::input::IrIn;
use std::num::ParseIntError;
use std::thread::sleep;
use std::time::Duration;
use structopt::StructOpt;

mod atmosphere;
mod ir;
mod lcd;
mod led;

fn parse_encoded(src: &str) -> Result<u128, ParseIntError> {
    u128::from_str_radix(src, 16)
}

#[derive(StructOpt, Debug)]
enum SendIrOpt {
    Raw {
        /// Decoded data
        bytes: Vec<u8>,
    },
    Encoded {
        /// Encoded data
        #[structopt(parse(try_from_str = parse_encoded))]
        hex: Vec<u128>,
    },
}

#[derive(StructOpt, Debug)]
enum IrOpt {
    Receive {
        /// Resend the signal after x seconds
        #[structopt(short, long)]
        resend: Option<usize>,
    },
    Send(SendIrOpt),
}

#[derive(StructOpt, Debug)]
enum Opt {
    Ir(IrOpt),
    Atmosphere {
        /// Number of readings
        #[structopt(short, long, default_value = "1")]
        times: usize,
    },
    Led {
        /// Which LED to use
        #[structopt(short, long)]
        led: Leds,

        /// Duration of on pulse in seconds
        #[structopt(short, long)]
        duration: u64,
    },
    Lcd {
        /// Text to display
        text: String,

        /// Duration of display in seconds
        #[structopt(short, long)]
        duration: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let opts = Opt::from_args();

    match opts {
        Opt::Ir(ir_opts) => match ir_opts {
            IrOpt::Receive { resend } => {
                let mut ir_in = IrIn::default_pin()?;
                let ir_stream = ir_in.pulse_stream();
                pin_mut!(ir_stream);
                let pulse_seq = ir_stream.next().await.unwrap().unwrap().unwrap();
                ir_in.stop().await?;
                println!(
                    "pulse sequence: {:?}",
                    pulse_seq
                        .0
                        .iter()
                        .map(|p| p.into_inner())
                        .collect::<Vec<_>>()
                );

                if let Some(re) = resend {
                    sleep(Duration::from_secs(re as u64));
                    let mut ir_out = IrOut::default_pin(Sanyo::default())?;
                    ir_out.send((*pulse_seq).clone())?;
                    sleep(Duration::from_secs(1));
                    println!("Finished sending!");
                    ir_out.stop()?;
                }
            }
            IrOpt::Send(send_opts) => match send_opts {
                SendIrOpt::Raw { bytes } => {
                    let mut ir_out = IrOut::default_pin(Sanyo::default())?;
                    ir_out.send(
                        <Sanyo as IrTarget>::Format::encode(bytes)
                            .wrap_err("Could not encode bytes")?,
                    )?;
                    sleep(Duration::from_secs(1));
                    println!("Finished sending!");
                    ir_out.stop()?;
                }
                SendIrOpt::Encoded { hex } => {
                    let mut ir_out = IrOut::default_pin(Sanyo::default())?;
                    ir_out.send(IrSequence(hex.into_iter().map(IrPulse).collect()))?;
                    sleep(Duration::from_secs(1));
                    println!("Finished sending!");
                    ir_out.stop()?;
                }
            },
        },
        Opt::Atmosphere { times } => {
            let atmo = Atmosphere::default_addr()?;
            let mut atmo_receiver = atmo.subscribe();
            for _ in 0..times {
                atmo_receiver.changed().await?;
                let reading = atmo_receiver.borrow();
                println!("atmosphere reading: {:?}", reading);
            }
            atmo.stop()?;
        }
        Opt::Led { led, duration } => {
            let mut led = Led::from_led(led)?;
            led.on();
            sleep(Duration::from_secs(duration));
            led.off();
        }
        Opt::Lcd { text, duration } => {
            let mut lcd = Lcd::default_addr()?;
            lcd.push_str(&text)?;
            lcd.wait_for_processing().await?;
            sleep(Duration::from_secs(duration));
            lcd.shutdown().await?;
        }
    }

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
    //     pulse_seq
    //         .0
    //         .iter()
    //         .map(|p| p.into_inner())
    //         .collect::<Vec<_>>()
    // );
    // println!("{}", Aeha::decode(pulse_seq.deref())?.to_string());
    // sleep(Duration::from_secs(3));
    // let mut out = IrOut::start(IR_OUTPUT_PIN, Sanyo::default())?;
    // let seq = (*pulse_seq).clone();
    // out.send(seq)?;
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

    // println!("starting 25 deg cool");
    // out.send_target(|t| {
    //     t.temp_set(SanyoTemperatureCode::T25)?;
    //     t.power_on()
    // })?;
    // sleep(Duration::from_secs(3));
    // println!("turning off");
    // out.send_target(IrTarget::power_off)?;
    // sleep(Duration::from_secs(2));
    //
    // out.stop()?;

    // let mut yellow_pin = Gpio::new()?.get(YELLOW_LED_PIN)?.into_output();
    // let mut green_pin = Gpio::new()?.get(GREEN_LED_PIN)?.into_output();

    Ok(())
}
