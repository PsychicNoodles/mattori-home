#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate log;

use eyre::WrapErr;
use futures_util::StreamExt;
use mattori_home_ui::client::{Client, ClientMessage};
use std::sync::mpsc;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;

    let (atmo_sender, atmo_receiver) = mpsc::channel();
    let (ac_status_sender, ac_status_receiver) = mpsc::channel();
    let (client_message_sender, client_message_receiver) = mpsc::channel();
    tokio::spawn(async move {
        let mut client = match Client::new(String::from("http://localhost:50051")).await {
            Ok(c) => c,
            Err(e) => {
                error!("Could not start home client: {}", e);
                return;
            }
        };
        let mut client_message_stream = match tokio::task::spawn_blocking(move || {
            futures_util::stream::repeat_with(move || client_message_receiver.recv())
        })
        .await
        {
            Ok(s) => s,
            Err(e) => {
                error!("Could not set up display message receiver: {}", e);
                return;
            }
        };

        let mut atmosphere_stream = match client.start_read_atmosphere().await {
            Ok(s) => s,
            Err(e) => {
                error!("Could not set up atmosphere reading stream: {}", e);
                return;
            }
        };
        tokio::spawn(async move {
            while let Ok(Some(reading)) = atmosphere_stream.message().await {
                if atmo_sender.send(reading).is_err() {
                    error!("Lost connection to atmosphere reading display");
                    break;
                }
            }
            debug!("Atmosphere reading stream closed");
        });

        while let Some(Ok(msg)) = client_message_stream.next().await {
            let res = match msg {
                ClientMessage::ChangeAtmosphereFeatures(features) => {
                    client.set_atmosphere_features(features)
                }
                ClientMessage::GetAcStatus => match client.get_ac_status().await {
                    Ok(status) => ac_status_sender
                        .send(status)
                        .wrap_err("Could not send AC status to display"),
                    Err(e) => Err(e),
                },
                ClientMessage::SetAcStatus(status) => match client.set_ac_status(status).await {
                    Ok(status) => ac_status_sender
                        .send(status)
                        .wrap_err("Could not send AC status to display"),
                    Err(e) => Err(e),
                },
                ClientMessage::Stop => break,
            };
            if let Err(e) = res {
                error!("Could not communicate with server: {}", e);
            }
        }
        debug!("Display client message stream closed");
    });

    let app =
        mattori_home_ui::HomeApp::new(atmo_receiver, ac_status_receiver, client_message_sender);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
