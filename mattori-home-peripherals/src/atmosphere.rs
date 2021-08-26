use std::sync::{mpsc, Mutex};
use std::thread::sleep;

use thiserror::Error;
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use tokio::time::{Duration, Instant};

use crate::atmosphere::types::{AtmoI2c, AtmoI2cError};
use std::fmt::{Display, Formatter};

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

impl Display for Reading {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let any = [
            self.temperature,
            self.pressure,
            self.humidity,
            self.altitude,
        ]
        .iter()
        .any(Option::is_some);
        if any {
            write!(f, "{{ ")?;
        }
        let mut has_prev = false;
        if let Some(t) = self.temperature {
            write!(f, "temperature: {}", t)?;
            has_prev = true;
        }
        if let Some(p) = self.pressure {
            if has_prev {
                write!(f, ", ")?;
            }
            write!(f, "pressure: {}", p)?;
            has_prev = true;
        }
        if let Some(h) = self.humidity {
            if has_prev {
                write!(f, ", ")?;
            }
            write!(f, "humidity: {}", h)?;
            has_prev = true;
        }
        if let Some(a) = self.altitude {
            if has_prev {
                write!(f, ", ")?;
            }
            write!(f, "altitude: {}", a)?;
        }
        if any {
            write!(f, " }}")?;
        }
        Ok(())
    }
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

#[derive(Clone, Debug)]
pub struct AtmosphereFeatures {
    pub temperature: bool,
    pub pressure: bool,
    pub humidity: bool,
    pub altitude: bool,
}

impl Default for AtmosphereFeatures {
    fn default() -> Self {
        Self {
            temperature: true,
            pressure: true,
            humidity: true,
            altitude: true,
        }
    }
}

impl AtmosphereFeatures {
    pub fn temperature_enabled(&self) -> bool {
        self.temperature || self.pressure_enabled() || self.humidity_enabled()
    }

    pub fn pressure_enabled(&self) -> bool {
        self.pressure || self.altitude_enabled()
    }

    pub fn humidity_enabled(&self) -> bool {
        self.humidity
    }

    pub fn altitude_enabled(&self) -> bool {
        self.altitude
    }
}

#[derive(Clone, Debug)]
pub enum ReaderMessage {
    Start,
    Pause,
    // todo implement
    ChangeEnabled(AtmosphereFeatures),
    Recalibrate,
    ChangeSeaLevelPressure(f32),
    Stop,
}

#[derive(Error, Clone, Debug)]
pub enum AtmosphereError {
    #[error(transparent)]
    Internal(#[from] AtmoI2cError),
    #[error("Could not communicate with i2c thread")]
    Send,
    #[error("Could not acquire message sender mutex")]
    Mutex,
}

pub type Result<T> = std::result::Result<T, AtmosphereError>;

#[derive(Debug)]
pub struct Atmosphere {
    reading_receiver: watch::Receiver<Result<Reading>>,
    message_sender: Mutex<mpsc::Sender<ReaderMessage>>,
}

impl Atmosphere {
    pub fn start(addr: u16) -> Result<Atmosphere> {
        let atmo_i2c = AtmoI2c::new(addr)?;
        let (message_sender, message_receiver) = mpsc::channel();
        let reading_receiver = Self::start_reading(atmo_i2c, message_receiver);

        Ok(Atmosphere {
            reading_receiver,
            message_sender: Mutex::new(message_sender),
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
            let mut features = AtmosphereFeatures::default();
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
                    trace!("skipping due to no reading receivers");
                    continue;
                }

                loop {
                    match message_receiver.try_recv() {
                        Ok(ReaderMessage::Stop) => {
                            info!("atmosphere thread received stop signal");
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => {
                            break;
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            info!("atmosphere stream message sender closed before stop signal");
                            return;
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
                        Ok(ReaderMessage::Recalibrate) => {
                            info!("atmosphere thread recalibrating");
                            if let Err(e) = atmo_i2c.reload_calibration() {
                                if reading_sender
                                    .send(Err(AtmosphereError::Internal(e)))
                                    .is_err()
                                {
                                    error!("could not trigger recalibration in atmosphere i2c");
                                }
                            }
                        }
                        Ok(ReaderMessage::ChangeSeaLevelPressure(pressure)) => {
                            info!(
                                "atmosphere thread changing sea level pressure to {}",
                                pressure
                            );
                            atmo_i2c.set_sea_level_pressure(pressure);
                        }
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
        features: &AtmosphereFeatures,
    ) -> Result<Reading> {
        Ok(if running && features.temperature_enabled() {
            trace!("running && temperature enabled");
            let (temp_fine, temperature) = atmo_i2c.read_temperature()?;
            trace!("read temperature: {:?} {:?}", temp_fine, temperature);

            let pressure = features
                .pressure_enabled()
                .then(|| atmo_i2c.read_pressure(temp_fine))
                .transpose()?;
            trace!("read pressure: {:?}", pressure);

            let humidity = features
                .humidity_enabled()
                .then(|| atmo_i2c.read_humidity(temp_fine))
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
            .lock()
            .map_err(|_| AtmosphereError::Mutex)?
            .send(ReaderMessage::Pause)
            .map_err(|_| AtmosphereError::Send)
    }

    pub fn restart(&self) -> Result<()> {
        self.message_sender
            .lock()
            .map_err(|_| AtmosphereError::Mutex)?
            .send(ReaderMessage::Start)
            .map_err(|_| AtmosphereError::Send)
    }

    pub fn stop(&self) -> Result<()> {
        self.message_sender
            .lock()
            .map_err(|_| AtmosphereError::Mutex)?
            .send(ReaderMessage::Stop)
            .map_err(|_| AtmosphereError::Send)
    }

    pub fn recalibrate(&self) -> Result<()> {
        self.message_sender
            .lock()
            .map_err(|_| AtmosphereError::Mutex)?
            .send(ReaderMessage::Recalibrate)
            .map_err(|_| AtmosphereError::Send)
    }

    pub fn change_sea_level_pressure(&self, pressure: f32) -> Result<()> {
        self.message_sender
            .lock()
            .map_err(|_| AtmosphereError::Mutex)?
            .send(ReaderMessage::ChangeSeaLevelPressure(pressure))
            .map_err(|_| AtmosphereError::Send)
    }
}
