use std::array::IntoIter;
use std::ops::Deref;
use std::pin::Pin;

use async_stream::stream;
use color_eyre::eyre::{eyre, WrapErr};
use eyre::Result;
use futures::{pin_mut, stream::Peekable, FutureExt, Stream, StreamExt};
use rppal::i2c::I2c;
use tokio::sync::watch;
use tokio::time::{self, Duration, Instant, Interval, MissedTickBehavior};
use tokio_stream::wrappers::IntervalStream;

use crate::atmosphere::commands::ReaderMessage;
use crate::atmosphere::types::{AtmoI2c, EnabledFeatures, Mode};
use std::sync::mpsc;
use std::thread::sleep;
use tokio::task::spawn_blocking;

mod calibration;
mod commands;
mod types;

const READ_RATE: Duration = Duration::from_secs(1);

pub struct Reading {
    pub temperature: Option<f32>,
    pub pressure: Option<f32>,
    pub humidity: Option<f32>,
    pub altitude: Option<f32>,
}

impl Reading {
    pub fn empty() -> Reading {
        Reading {
            temperature: None,
            pressure: None,
            humidity: None,
            altitude: None,
        }
    }
}

pub struct Atmosphere {
    reading_receiver: watch::Receiver<Result<Reading>>,
    mode_sender: mpsc::Sender<ReaderMessage>,
}

impl Atmosphere {
    pub fn start(addr: u16) -> Result<Atmosphere> {
        let mut atmo_i2c = AtmoI2c::new(addr)?;
        let (mode_sender, mode_receiver) = mpsc::channel();
        let reading_receiver = Self::start_reading(atmo_i2c, mode_receiver);

        Ok(Atmosphere {
            reading_receiver,
            mode_sender,
        })
    }

    fn start_reading(
        mut atmo_i2c: AtmoI2c,
        mut message_receiver: mpsc::Receiver<ReaderMessage>,
    ) -> watch::Receiver<Result<Reading>> {
        let (reading_sender, reading_receiver) = watch::channel(Ok(Reading::empty()));

        spawn_blocking(move || {
            let mut features = EnabledFeatures::default();
            let mut running = true;
            let mut next_tick = Instant::now() + READ_RATE;
            loop {
                let now = Instant::now();
                if now < next_tick {
                    sleep(next_tick - now);
                } else {
                    info!("next tick already surpassed, might need to increase read rate");
                }
                next_tick = next_tick + READ_RATE;

                if reading_sender.receiver_count() == 0 {
                    debug!("skipping due to no reading receivers");
                    continue;
                }

                match message_receiver.recv() {
                    Ok(ReaderMessage::Stop) => break,
                    Err(_) => {
                        info!("atmosphere stream message sender closed before stop signal");
                        break;
                    }
                    Ok(ReaderMessage::Pause) => running = false,
                    Ok(ReaderMessage::ChangeEnabled(new_features)) => features = new_features,
                    Ok(ReaderMessage::Start) => running = true,
                }

                let reading = Self::perform_reading(&mut atmo_i2c, running, &features);

                reading_sender.send(reading);
            }
        });

        reading_receiver
    }

    fn perform_reading(
        atmo_i2c: &mut AtmoI2c,
        running: bool,
        features: &EnabledFeatures,
    ) -> Result<Reading> {
        Ok(if running && features.temperature_enabled() {
            let (temp_fine, temperature) = atmo_i2c
                .read_temperature()
                .wrap_err("Could not get temperature reading")?;

            let pressure = features
                .pressure_enabled()
                .then(|| {
                    atmo_i2c
                        .read_pressure(temp_fine)
                        .wrap_err("Could not get pressure reading")
                })
                .transpose()?;

            let humidity = features
                .humidity_enabled()
                .then(|| {
                    atmo_i2c
                        .read_humidity(temp_fine)
                        .wrap_err("Could not get humidity reading")
                })
                .transpose()?;

            let altitude = pressure.and_then(|p| {
                features
                    .altitude_enabled()
                    .then(|| atmo_i2c.read_altitude(p))
            });

            Reading {
                temperature: Some(temperature),
                pressure,
                humidity,
                altitude,
            }
        } else {
            Reading::empty()
        })
    }

    pub fn subscribe(&self) -> watch::Receiver<Result<Reading>> {
        self.reading_receiver.clone()
    }
}
