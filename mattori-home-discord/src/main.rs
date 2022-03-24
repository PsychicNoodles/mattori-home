use std::{str::FromStr, sync::Arc};

use mattori_home_peripherals::{
    atmosphere::{Atmosphere, Reading},
    ir::{sanyo::types::SanyoTemperatureCode, types::ACMode},
};
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

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

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

#[derive(Debug)]
enum Commands {
    Atmosphere,
    PowerOn,
    PowerOff,
}

impl Commands {
    fn message(self, m: &mut CreateMessage) {
        match self {
            Commands::Atmosphere => todo!(),
            Commands::PowerOn => todo!(),
            Commands::PowerOff => todo!(),
        }
    }
}

#[derive(Error, Debug)]
#[error("Failed to parse {0} as command")]
struct CommandParseError(String);

impl FromStr for Commands {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Commands::*;
        match s {
            "atmosphere" | "atmo" => Ok(Atmosphere),
            "poweron" | "on" => Ok(PowerOn),
            "poweroff" | "off" => Ok(PowerOff),
            _ => Err(CommandParseError(s.to_string())),
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        trace!("message: {:?}", msg);

        match Commands::from_str(&msg.content) {
            Ok(c) => match c {
                Commands::Atmosphere => {
                    let m = msg
                        .channel_id
                        .send_message(&ctx, |m| {
                            m.content("Choose a mode and temperature");
                            let mut ar = CreateActionRow::default();
                            let mut mode_sm = CreateSelectMenu::default();
                            mode_sm.placeholder("Mode");
                            mode_sm.options(|opts| {
                                ACMode::iter().for_each(|mode| {
                                    opts.create_option(|o| {
                                        o.label(mode.to_string());
                                        o.value(mode.to_string());
                                        o
                                    });
                                });
                                opts
                            });
                            ar.add_select_menu(mode_sm);
                            let mut temp_sm = CreateSelectMenu::default();
                            temp_sm.placeholder("Temperature");
                            temp_sm.options(|opts| {
                                SanyoTemperatureCode::iter().for_each(|temp| {
                                    opts.create_option(|o| {
                                        let t = u32::from(temp).to_string();
                                        o.label(format!("{}°", t));
                                        o.label(t);
                                        o
                                    });
                                });
                                opts
                            });
                            m.components(|c| {
                                c.add_action_row(ar);
                                c
                            });
                            m
                        })
                        .await
                        .unwrap();
                }
                Commands::PowerOn => todo!(),
                Commands::PowerOff => todo!(),
            },
            Err(_) => return,
        }
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
