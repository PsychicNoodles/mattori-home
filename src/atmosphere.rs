use color_eyre::eyre::WrapErr;
use eyre::Result;

use tokio::sync::watch;
use tokio::time::{Duration, Instant};

use crate::atmosphere::commands::ReaderMessage;
use crate::atmosphere::types::{AtmoI2c, EnabledFeatures};
use std::sync::mpsc;
use std::thread::sleep;
use tokio::task::spawn_blocking;

mod calibration;
mod commands;
mod types;

const ATMOSPHERE_ADDR: u16 = 0x76;

const READ_RATE: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
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
    message_sender: mpsc::Sender<ReaderMessage>,
}

impl Atmosphere {
    pub fn start(addr: u16) -> Result<Atmosphere> {
        let atmo_i2c = AtmoI2c::new(addr)?;
        let (message_sender, message_receiver) = mpsc::channel();
        let reading_receiver = Self::start_reading(atmo_i2c, message_receiver);

        Ok(Atmosphere {
            reading_receiver,
            message_sender,
        })
    }

    pub fn default_addr() -> Result<Self> {
        Self::start(ATMOSPHERE_ADDR)
    }

    fn start_reading(
        mut atmo_i2c: AtmoI2c,
        message_receiver: mpsc::Receiver<ReaderMessage>,
    ) -> watch::Receiver<Result<Reading>> {
        let (reading_sender, reading_receiver) = watch::channel(Ok(Reading::empty()));

        spawn_blocking(move || {
            let mut features = EnabledFeatures::default();
            let mut running = true;
            let mut next_tick = Instant::now() + READ_RATE;
            loop {
                let now = Instant::now();
                if now < next_tick {
                    trace!("sleeping {:?}", next_tick - now);
                    sleep(next_tick - now);
                } else {
                    info!("next tick already surpassed, might need to increase read rate");
                }
                next_tick += READ_RATE;

                if reading_sender.receiver_count() <= 1 {
                    debug!("skipping due to no reading receivers");
                    continue;
                }

                match message_receiver.try_recv() {
                    Ok(ReaderMessage::Stop) => {
                        info!("atmosphere thread received stop signal");
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        // no new messages so skip
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        info!("atmosphere stream message sender closed before stop signal");
                        break;
                    }
                    Ok(ReaderMessage::Pause) => {
                        info!("atmosphere thread pausing");
                        running = false
                    }
                    Ok(ReaderMessage::ChangeEnabled(new_features)) => {
                        info!(
                            "atmosphere thread switching to new enabled features: {:?}",
                            new_features
                        );
                        features = new_features
                    }
                    Ok(ReaderMessage::Start) => {
                        info!("atmosphere thread starting");
                        running = true
                    }
                }

                let reading = Self::perform_reading(&mut atmo_i2c, running, &features);

                if reading_sender.send(reading).is_err() {
                    info!("sent to no reading receivers");
                }
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
            trace!("running && temperature enabled");
            let (temp_fine, temperature) = atmo_i2c
                .read_temperature()
                .wrap_err("Could not get temperature reading")?;
            trace!("read temperature: {:?} {:?}", temp_fine, temperature);

            let pressure = features
                .pressure_enabled()
                .then(|| {
                    atmo_i2c
                        .read_pressure(temp_fine)
                        .wrap_err("Could not get pressure reading")
                })
                .transpose()?;
            trace!("read pressure: {:?}", pressure);

            let humidity = features
                .humidity_enabled()
                .then(|| {
                    atmo_i2c
                        .read_humidity(temp_fine)
                        .wrap_err("Could not get humidity reading")
                })
                .transpose()?;
            trace!("read humidity: {:?}", humidity);

            let altitude = pressure.and_then(|p| {
                features
                    .altitude_enabled()
                    .then(|| atmo_i2c.read_altitude(p))
            });
            trace!("read altitude: {:?}", altitude);

            Reading {
                temperature: Some(temperature),
                pressure,
                humidity,
                altitude,
            }
        } else {
            trace!("skip reading");
            Reading::empty()
        })
    }

    pub fn subscribe(&self) -> watch::Receiver<Result<Reading>> {
        self.reading_receiver.clone()
    }

    pub fn pause(&self) -> Result<()> {
        self.message_sender
            .send(ReaderMessage::Pause)
            .wrap_err("Could not send pause message to atmosphere reading thread")
    }

    pub fn restart(&self) -> Result<()> {
        self.message_sender
            .send(ReaderMessage::Start)
            .wrap_err("Could not send (re)start message to atmosphere reading thread")
    }

    pub fn stop(&self) -> Result<()> {
        self.message_sender
            .send(ReaderMessage::Stop)
            .wrap_err("Could not send stop message to atmosphere reading thread")
    }
}
