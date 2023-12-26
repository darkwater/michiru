use egui::{CollapsingHeader, Grid, ScrollArea};
use egui_json_tree::{DefaultExpand, JsonTree};
use michiru_zigbee2mqtt::definitions::{
    DeviceDefinition, DeviceInfo, Expose, Feature, FeatureMeta,
};
use serde::{Deserialize, Serialize};

use crate::{
    state::{AppState, InvalidZigbee2mqttDevice},
    topic_tree::TopicPayload,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Zigbee2Mqtt {
    selected: Option<Selection>,
}

#[derive(Debug, Hash, PartialEq, Serialize, Deserialize)]
pub enum Selection {
    Index(usize),
    Address(String),
}

impl Zigbee2Mqtt {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        egui::SidePanel::left("tree").show_inside(ui, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (idx, device) in state.zigbee2mqtt_devices.iter().enumerate() {
                        match device {
                            Ok(device) => {
                                ui.selectable_value(
                                    &mut self.selected,
                                    Some(Selection::Address(device.ieee_address.clone())),
                                    device.model_id.clone(),
                                );
                            }
                            Err(InvalidZigbee2mqttDevice { error, .. }) => {
                                ui.selectable_value(
                                    &mut self.selected,
                                    Some(Selection::Index(idx)),
                                    error.to_string(),
                                );
                            }
                        }
                    }
                });
        });

        let Some(selected) = self.selected.as_ref().and_then(|s| match s {
            Selection::Index(idx) => state.zigbee2mqtt_devices.get(*idx),
            Selection::Address(addr) => {
                state
                    .zigbee2mqtt_devices
                    .iter()
                    .find(|device| match device {
                        Ok(device) => device.ieee_address == *addr,
                        Err(_) => false,
                    })
            }
        }) else {
            return;
        };

        egui::SidePanel::right("current state").show_inside(ui, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let Ok(selected) = selected else {
                        return;
                    };

                    let Some(device) = state
                        .topic_tree
                        .get(&format!("zigbee2mqtt/{}", selected.ieee_address))
                    else {
                        return;
                    };

                    match &device.payload {
                        TopicPayload::Json(v) => {
                            JsonTree::new(&selected.ieee_address, v)
                                .default_expand(DefaultExpand::ToLevel(2))
                                .show(ui);
                        }
                        _ => {
                            ui.label("Not JSON");
                        }
                    }
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| match selected {
                    Ok(device) => {
                        device_ui(device, ui);
                    }
                    Err(InvalidZigbee2mqttDevice { json, error }) => {
                        ui.label(format!("Error: {}", error));

                        JsonTree::new(&self.selected, json)
                            .default_expand(DefaultExpand::ToLevel(2))
                            .show(ui);
                    }
                });
        });
    }
}

fn device_ui(device: &DeviceInfo, ui: &mut egui::Ui) {
    let DeviceInfo {
        friendly_name,
        ieee_address,
        model_id,
        manufacturer,
        power_source,
        zigbee_device_type,
        interview_completed,
        disabled,
        definition: DeviceDefinition { description, exposes },
    } = &device;

    ui.heading(friendly_name);

    Grid::new("details")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("IEEE Address");
            ui.label(ieee_address);
            ui.end_row();

            ui.strong("Model ID");
            ui.label(model_id);
            ui.end_row();

            ui.strong("Manufacturer");
            ui.label(manufacturer);
            ui.end_row();

            ui.strong("Power Source");
            ui.label(power_source);
            ui.end_row();

            ui.strong("Zigbee Device Type");
            ui.label(zigbee_device_type.to_string());
            ui.end_row();

            ui.strong("Interview Completed");
            ui.label(if *interview_completed { "Yes" } else { "No" });
            ui.end_row();

            ui.strong("Disabled");
            ui.label(if *disabled { "Yes" } else { "No" });
            ui.end_row();

            ui.strong("Description");
            ui.label(description);
            ui.end_row();
        });

    for expose in exposes {
        ui.add_space(20.);

        match expose {
            Expose::Generic(feature) => {
                feature_ui(feature, ui);
            }
            Expose::Specific(specific) => {
                CollapsingHeader::new(specific.ty.to_string())
                    .default_open(true)
                    .show(ui, |ui| {
                        for feature in &specific.features {
                            feature_ui(feature, ui);
                            ui.add_space(20.);
                        }
                    });
            }
        }
    }
}

fn feature_meta_ui(meta: &FeatureMeta, ui: &mut egui::Ui) {
    if meta.name == meta.property {
        ui.strong(&meta.name);
    } else {
        ui.strong(format!("{} (property: {})", meta.name, meta.property));
    }

    if let Some(ref description) = meta.description {
        ui.label(description);
    }

    let mut access = vec![];
    if meta.access.published {
        access.push("Published");
    }
    if meta.access.settable {
        access.push("Settable");
    }
    if meta.access.gettable {
        access.push("Gettable");
    }

    ui.label(format!("Access: {}", access.join(", ")));
}

fn feature_ui(feature: &Feature, ui: &mut egui::Ui) {
    feature_meta_ui(feature.meta(), ui);

    ui.label(format!("Type: {}", feature.ty()));

    match feature {
        Feature::Binary {
            meta: _,
            value_on,
            value_off,
            value_toggle,
        } => {}
        Feature::Numeric {
            meta: _,
            value_min,
            value_max,
            value_step,
            unit,
            presets,
        } => {
            if let Some(unit) = unit {
                ui.label(format!("Unit: {}", unit));
            }
        }
        Feature::Text { meta: _ } => {}
        Feature::Enum { meta: _, values } => {
            ui.label(format!("Values: {}", values.join(", ")));
        }
        Feature::Composite { meta, features } => {
            ui.horizontal(|ui| {
                ui.add_space(20.);
                ui.vertical(|ui| {
                    for feature in features {
                        ui.add_space(20.);
                        feature_ui(feature, ui);
                    }
                });
            });
        }
        Feature::List => {}
    }
}
