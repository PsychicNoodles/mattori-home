mod commands;

use std::{str::FromStr, time::Duration};

use mattori_home_peripherals::atmosphere::{Atmosphere, Reading};
use serenity::{
    async_trait,
    builder::{CreateActionRow, CreateMessage, CreateSelectMenu},
    client::{Context, EventHandler},
    futures::StreamExt,
    model::channel::{Message, ReactionType},
    Client,
};
use strum::IntoEnumIterator;
use thiserror::Error;
use tokio::{
    select, spawn,
    sync::{broadcast, mpsc},
};

use crate::commands::Commands;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

const INTERACTION_TIMEOUT: Duration = Duration::from_secs(60 * 3);

async fn ir_read_loop(
    cmd_tx: mpsc::UnboundedSender<Reading>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let atmo = match Atmosphere::default_addr() {
        Err(e) => {
            error!("could not start reading atmosphere data: {}", e);
            return;
        }
        Ok(v) => v,
    };
    let mut atmo_receiver = atmo.subscribe();

    loop {
        select! {
            _ = shutdown_rx.recv() => {
                info!("received shutdown");
                break;
            },
            change = atmo_receiver.changed() => {
                match change {
                    Ok(()) => {
                        let val = atmo_receiver.borrow_and_update().clone();
                        match val {
                            Err(e) => {
                                error!("error reading atmosphere data: {}", e);
                                break;
                            },
                            Ok(v) => {
                                trace!("atmosphere data received: {}", v);
                                if let Err(e) = cmd_tx.send(v) {
                                    error!("could not send atmosphere data to main thread: {}", e);
                                    break;
                                }
                            }
                        }
                    },
                    Err(_) => {
                        error!("atmosphere data sender closed");
                        break;
                    }
                }
            }
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        trace!("message: {:?}", msg);

        let mut cmd = match Commands::from_str(&msg.content) {
            Ok(c) => c,
            Err(e) => {
                error!("could not parse command: {}", e);
                return;
            }
        };

        let m = msg
            .channel_id
            .send_message(&ctx, |m| {
                cmd.create_message(m);
                m
            })
            .await
            .unwrap();

        cmd.collect_interactions(
            &ctx,
            m.await_component_interactions(&ctx)
                .timeout(INTERACTION_TIMEOUT),
        )
        .await;
    }
}

#[tokio::main]
pub async fn main() {
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

    spawn(async move {
        ir_read_loop(cmd_tx, shutdown_rx).await;
    });
}
