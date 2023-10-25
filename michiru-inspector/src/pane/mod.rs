mod homie_devices;
mod mqtt_topics;
mod zigbee2mqtt;

use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub enum Pane {
    MqttTopics,
    HomieDevices,
    Zigbee2Mqtt(zigbee2mqtt::Zigbee2Mqtt),
}

impl Pane {
    pub fn all() -> Vec<Pane> {
        vec![Pane::MqttTopics, Pane::HomieDevices, Pane::Zigbee2Mqtt(Default::default())]
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        match self {
            Pane::MqttTopics => mqtt_topics::MqttTopics.ui(ui, state),
            Pane::HomieDevices => homie_devices::HomieDevices.ui(ui, state),
            Pane::Zigbee2Mqtt(p) => p.ui(ui, state),
        }
    }
}
