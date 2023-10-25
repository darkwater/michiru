use itertools::Itertools;
use michiru_zigbee2mqtt::definitions::DeviceInfo;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::topic_tree::{TopicPayload, TopicTree, TopicValue};

pub struct AppState {
    pub topic_rx: UnboundedReceiver<TopicValue>,
    pub topic_tree: TopicTree,
    pub selected: Option<TopicValue>,
    pub zigbee2mqtt_devices: Vec<Result<DeviceInfo, InvalidZigbee2mqttDevice>>,
}

#[derive(Debug)]
pub struct InvalidZigbee2mqttDevice {
    pub json: serde_json::Value,
    pub error: serde_json::Error,
}

impl AppState {
    pub fn new(topic_rx: UnboundedReceiver<TopicValue>) -> Self {
        Self {
            topic_rx,
            topic_tree: TopicTree::default(),
            selected: None,
            zigbee2mqtt_devices: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        while let Ok(value) = self.topic_rx.try_recv() {
            if value.topic == "zigbee2mqtt/bridge/devices" {
                if let TopicPayload::Json(serde_json::Value::Array(devices)) = &value.payload {
                    self.zigbee2mqtt_devices = devices
                        .iter()
                        .map(|json| {
                            serde_json::from_value::<DeviceInfo>(json.clone()).map_err(|error| {
                                InvalidZigbee2mqttDevice { json: json.clone(), error }
                            })
                        })
                        .collect_vec();
                }
            }

            self.topic_tree.insert(value);
        }
    }
}
