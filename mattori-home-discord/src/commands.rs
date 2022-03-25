use std::str::FromStr;

use async_trait::async_trait;
use serenity::{
    builder::CreateMessage, client::Context, collector::ComponentInteractionCollectorBuilder,
};
use thiserror::Error;

pub mod atmosphere;

#[derive(Debug)]
pub enum Commands {
    Atmosphere(atmosphere::Atmosphere),
    PowerOn,
    PowerOff,
}

#[derive(Error, Debug)]
#[error("Failed to parse {0} as command")]
pub struct CommandParseError(String);

impl FromStr for Commands {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "atmosphere" | "atmo" => Ok(Commands::Atmosphere(atmosphere::Atmosphere::default())),
            "poweron" | "on" => Ok(Commands::PowerOn),
            "poweroff" | "off" => Ok(Commands::PowerOff),
            _ => Err(CommandParseError(s.to_string())),
        }
    }
}

#[async_trait]
trait Command {
    fn create_message(&self, m: &mut CreateMessage);
    async fn collect_interactions<'a>(
        &mut self,
        context: &Context,
        interactions: ComponentInteractionCollectorBuilder<'a>,
    );
}

impl Commands {
    pub fn create_message(&self, m: &mut CreateMessage) {
        match self {
            Commands::Atmosphere(a) => a.create_message(m),
            Commands::PowerOn => todo!(),
            Commands::PowerOff => todo!(),
        };
    }

    pub async fn collect_interactions<'a>(
        &mut self,
        context: &Context,
        interactions: ComponentInteractionCollectorBuilder<'a>,
    ) {
        match self {
            Commands::Atmosphere(a) => a.collect_interactions(context, interactions),
            Commands::PowerOn => todo!(),
            Commands::PowerOff => todo!(),
        }
        .await;
    }
}
