use std::str::FromStr;

use async_trait::async_trait;
use mattori_home_peripherals::ir::{sanyo::types::SanyoTemperatureCode, types::ACMode};
use serenity::{
    builder::{CreateActionRow, CreateButton, CreateSelectMenu},
    client::Context,
    futures::StreamExt,
};
use strum::IntoEnumIterator;

use super::Command;

#[derive(Debug, Default)]
pub struct Atmosphere {
    mode: Option<ACMode>,
    temp: Option<SanyoTemperatureCode>,
}

impl Atmosphere {
    const MODE_ID: &'static str = "mode_sm";
    const TEMP_ID: &'static str = "temp_sm";
    const CONF_ID: &'static str = "cfm_btn";
}

#[async_trait]
impl Command for Atmosphere {
    fn create_message(&self, m: &mut serenity::builder::CreateMessage) {
        m.content("Choose a mode and temperature");
        let mut ar = CreateActionRow::default();
        let mut mode_sm = CreateSelectMenu::default();
        mode_sm.custom_id(Atmosphere::MODE_ID);
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
        temp_sm.custom_id(Atmosphere::TEMP_ID);
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
        let mut cfm = CreateButton::default();
        cfm.label("Confirm");
        cfm.custom_id(Atmosphere::CONF_ID);
        ar.add_button(cfm);
        m.components(|c| {
            c.add_action_row(ar);
            c
        });
    }

    async fn collect_interactions<'a>(
        &mut self,
        context: &Context,
        interactions: serenity::collector::ComponentInteractionCollectorBuilder<'a>,
    ) {
        let mut ints = interactions.await;

        while let Some(int) = ints.next().await {
            if int.data.custom_id == Atmosphere::CONF_ID {
                if self.mode.is_none() || self.temp.is_none() {
                    if let Err(e) = int
                        .create_followup_message(context, |f| {
                            f.content("You must choose a mode and temperature first");
                            f
                        })
                        .await
                    {
                        error!("could not send follow up message: {}", e);
                        return;
                    }
                } else {
                    return;
                }
            } else {
                match int.data.custom_id.as_str() {
                    Atmosphere::MODE_ID => match ACMode::from_str(&int.data.values[0]) {
                        Ok(m) => {
                            let _ = self.mode.insert(m);
                        }
                        Err(e) => {
                            error!("could not parse mode: {}", e);
                            return;
                        }
                    },
                    Atmosphere::TEMP_ID => {
                        match SanyoTemperatureCode::from_str(&int.data.values[0]) {
                            Ok(t) => {
                                let _ = self.temp.insert(t);
                            }
                            Err(e) => {
                                error!("could not parse temp: {}", e);
                                return;
                            }
                        }
                    }
                    data => {
                        error!("unexpected custom_id: {}", data);
                        return;
                    }
                }
            }
        }
    }
}
