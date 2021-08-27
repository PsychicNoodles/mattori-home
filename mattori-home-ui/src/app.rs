use crate::client::mattori_home::{AcStatus, AtmosphereReading};
use crate::client::ClientMessage;
use eframe::{egui, epi};
use std::sync::mpsc;

pub struct HomeApp {
    atmo_receiver: mpsc::Receiver<AtmosphereReading>,
    latest_atmo: Option<AtmosphereReading>,
    ac_status_receiver: mpsc::Receiver<AcStatus>,
    latest_ac_status: Option<AcStatus>,
    client_message_sender: mpsc::Sender<ClientMessage>,
}

impl HomeApp {
    pub fn new(
        atmo_receiver: mpsc::Receiver<AtmosphereReading>,
        ac_status_receiver: mpsc::Receiver<AcStatus>,
        client_message_sender: mpsc::Sender<ClientMessage>,
    ) -> HomeApp {
        HomeApp {
            atmo_receiver,
            latest_atmo: None,
            ac_status_receiver,
            latest_ac_status: None,
            client_message_sender,
        }
    }
}

impl epi::App for HomeApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self {
            atmo_receiver,
            latest_atmo,
            ac_status_receiver,
            latest_ac_status,
            client_message_sender,
        } = self;

        if let Ok(atmo) = atmo_receiver.try_recv() {
            let _ = latest_atmo.insert(atmo);
        }
        if let Ok(ac) = ac_status_receiver.try_recv() {
            let _ = latest_ac_status.insert(ac);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui template");
            ui.hyperlink("https://github.com/emilk/egui_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/egui_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }

    // /// Called by the framework to load old app state (if any).
    // fn setup(
    //     &mut self,
    //     _ctx: &egui::CtxRef,
    //     _frame: &mut epi::Frame<'_>,
    //     storage: Option<&dyn epi::Storage>,
    // ) {
    //     if let Some(storage) = storage {
    //         *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    //     }
    // }
    //
    // /// Called by the frame work to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn epi::Storage) {
    //     epi::set_value(storage, epi::APP_KEY, self);
    // }

    fn name(&self) -> &str {
        "egui template"
    }
}
