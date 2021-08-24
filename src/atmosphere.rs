mod calibration;
mod commands;
mod types;

use crate::atmosphere::commands::ReaderMessage;
use crate::atmosphere::types::{AtmoI2c, Mode};
use async_stream::stream;
use color_eyre::eyre::{eyre, WrapErr};
use eyre::Result;
use futures::{pin_mut, stream::Peekable, FutureExt, Stream, StreamExt};
use rppal::i2c::I2c;
use std::array::IntoIter;
use std::ops::Deref;
use std::pin::Pin;
use tokio::sync::{mpsc, watch};
use tokio::time::{self, Duration, Instant, Interval, MissedTickBehavior};
use tokio_stream::wrappers::IntervalStream;

pub struct Atmosphere {
    // altitude_receiver: watch::Receiver<f64>,
    // humidity_receiver: watch::Receiver<f64>,
    // pressure_receiver: watch::Receiver<f64>,
    // temperature_receiver: watch::Receiver<f64>,
    mode_sender: mpsc::UnboundedSender<ReaderMessage>,
}

impl Atmosphere {
    pub fn start(addr: u16) -> Result<Atmosphere> {
        let atmo_i2c = AtmoI2c::new(addr)?;
        let (altitude_sender, altitude_receiver) = watch::channel(f64::default());
        let (humidity_sender, humidity_receiver) = watch::channel(f64::default());
        let (pressure_sender, pressure_receiver) = watch::channel(f64::default());
        let (temperature_sender, temperature_receiver) = watch::channel(f64::default());
        let (mode_sender, mut mode_receiver) = mpsc::unbounded_channel();

        Ok(Atmosphere {
            // altitude_receiver,
            // humidity_receiver,
            // pressure_receiver,
            // temperature_receiver,
            mode_sender,
        })
    }
}
