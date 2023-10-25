use chrono::Local;
use egui::ScrollArea;
use egui_json_tree::{DefaultExpand, JsonTree};

use crate::{state::AppState, topic_tree::TopicPayload};

pub struct MqttTopics;

impl MqttTopics {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        egui::SidePanel::left("tree").show_inside(ui, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    state.topic_tree.show(ui, &mut state.selected);
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(value) = &state
                        .selected
                        .as_ref()
                        .and_then(|v| state.topic_tree.get(&v.topic))
                    {
                        let seconds = Local::now()
                            .signed_duration_since(value.received)
                            .num_seconds();

                        ui.heading(&value.topic);
                        ui.add_space(5.);
                        ui.label(format!("{seconds} seconds ago"));
                        ui.add_space(5.);
                        match &value.payload {
                            TopicPayload::Json(v) => {
                                JsonTree::new(&value.topic, v)
                                    .default_expand(DefaultExpand::ToLevel(0))
                                    .show(ui);
                            }
                            TopicPayload::String(v) => {
                                ui.label(v);
                            }
                            TopicPayload::Bytes(v) => {
                                ui.label(format!("{:02x?}", v));
                            }
                        };
                    }
                });
        });
    }
}
