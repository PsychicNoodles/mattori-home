mod conversions;
mod server;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use crate::server::{mattori_home::home_server::HomeServer, HomeImpl};
use color_eyre::eyre::WrapErr;
use mattori_home_peripherals::atmosphere::Atmosphere;
use mattori_home_peripherals::ir::format::Aeha;
use mattori_home_peripherals::ir::input::IrIn;
use mattori_home_peripherals::ir::output::IrOut;
use mattori_home_peripherals::ir::sanyo::types::SanyoTemperatureCode;
use mattori_home_peripherals::ir::sanyo::Sanyo;
use mattori_home_peripherals::ir::types::{ACMode, IrFormat, IrPulse, IrSequence, IrTarget};
use mattori_home_peripherals::lcd::Lcd;
use mattori_home_peripherals::led::{Led, Leds};
use std::net::SocketAddr;
use std::num::ParseIntError;
use std::thread::sleep;
use std::time::Duration;
use structopt::StructOpt;
use tokio::pin;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tonic::transport::Server;

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
    Registered(AcState),
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
struct AcState {
    #[structopt(short, long)]
    unpowered: bool,
    #[structopt(short, long, default_value = "cool")]
    mode: ACMode,
    #[structopt(short, long, default_value = "25")]
    temperature: SanyoTemperatureCode,
}

impl AcState {
    fn send(
        &self,
        target: &mut Sanyo,
    ) -> std::result::Result<IrSequence, <Sanyo as IrTarget>::Error> {
        target.mode_set(self.mode.clone())?;
        if let Some(res) = target.temp_set(self.temperature.clone()) {
            res?;
        }
        if self.unpowered {
            target.power_off()
        } else {
            target.power_on()
        }
    }
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
    Server {
        /// Address for server
        #[structopt(short, long, default_value = "[::1]:50051")]
        addr: SocketAddr,

        #[structopt(flatten)]
        initial_state: AcState,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    color_eyre::install()?;

    let opts = Opt::from_args();

    debug!("opts: {:?}", opts);

    match opts {
        Opt::Ir(ir_opts) => match ir_opts {
            IrOpt::Receive { resend } => {
                let mut ir_in = IrIn::default_pin()?;
                let ir_stream = ir_in.pulse_stream();
                pin!(ir_stream);
                let pulse_seq = ir_stream.next().await.unwrap().unwrap().unwrap();
                ir_in.stop().await?;
                println!("Received pulse sequence: {}", pulse_seq.as_hex::<Aeha>()?);

                if let Some(re) = resend {
                    sleep(Duration::from_secs(re as u64));
                    let mut ir_out = IrOut::default_pin(Sanyo::default())?;
                    ir_out.send((*pulse_seq).clone())?;
                    sleep(Duration::from_secs(1));
                    println!("Finished sending!");
                    ir_out.stop()?;
                }
            }
            IrOpt::Send(send_opts) => {
                let mut ir_out = IrOut::default_pin(Sanyo::default())?;
                match send_opts {
                    SendIrOpt::Raw { bytes } => ir_out.send(
                        <Sanyo as IrTarget>::Format::encode(bytes)
                            .wrap_err("Could not encode bytes")?,
                    )?,
                    SendIrOpt::Encoded { hex } => {
                        ir_out.send(IrSequence(hex.into_iter().map(IrPulse).collect()))?
                    }
                    SendIrOpt::Registered(state) => ir_out.send_target(|o| state.send(o))?,
                }
                sleep(Duration::from_secs(1));
                println!("Finished sending!");
                ir_out.stop()?;
            }
        },
        Opt::Atmosphere { times } => {
            let atmo = Atmosphere::default_addr()?;
            let mut atmo_receiver = atmo.subscribe();
            for _ in 0..times {
                atmo_receiver.changed().await?;
                let reading = atmo_receiver.borrow();
                println!("Atmosphere reading: {}", reading.clone()?);
            }
            atmo.stop()?;
        }
        Opt::Led { led, duration } => {
            let mut led = Led::from_led(led)?;
            println!("Turning on led...");
            led.on();
            sleep(Duration::from_secs(duration));
            led.off();
            println!("Turned off led");
        }
        Opt::Lcd { text, duration } => {
            let mut lcd = Lcd::default_addr()?;
            println!("Displaying text: {}", text);
            lcd.push_str(&text)?;
            lcd.wait_for_processing().await?;
            sleep(Duration::from_secs(duration));
            println!("Clearing lcd");
            lcd.shutdown().await?;
        }
        Opt::Server {
            addr,
            initial_state,
        } => {
            let mut out = IrOut::default_pin(Sanyo::default())?;
            out.send_target(|o| initial_state.send(o))?;
            let home = HomeImpl {
                atmosphere: Atmosphere::default_addr()?,
                ir_out: Mutex::new(out),
            };

            println!("Starting server at {}", addr);

            Server::builder()
                .add_service(HomeServer::new(home))
                .serve(addr)
                .await?;
        }
    }

    Ok(())
}
