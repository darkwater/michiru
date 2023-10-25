pub mod definitions;

use anyhow::Result;
use rumqttc::{AsyncClient, ConnectionError, Event, EventLoop, MqttOptions, Packet, QoS};

use crate::definitions::DeviceInfo;

pub struct DefinitionStream {
    eventloop: EventLoop,
}

impl DefinitionStream {
    pub async fn new(options: MqttOptions) -> Self {
        let (client, eventloop) = AsyncClient::new(options, 10);
        client
            .subscribe("zigbee2mqtt/bridge/devices", QoS::ExactlyOnce)
            .await
            .unwrap();

        Self { eventloop }
    }

    pub async fn next(&mut self) -> Result<Vec<DeviceInfo>, ConnectionError> {
        loop {
            let notification = self.eventloop.poll().await?;

            tracing::trace!(?notification);

            let Event::Incoming(Packet::Publish(obj)) = notification else {
                continue;
            };

            let Ok(devices) = serde_json::from_slice::<Vec<serde_json::Value>>(&obj.payload) else {
                continue;
            };

            return Ok(devices
                .iter()
                .filter_map(|device| match serde_json::from_value::<DeviceInfo>(device.clone()) {
                    Ok(device) => {
                        // tracing::info!("{devices:#?}");
                        Some(device)
                    }
                    Err(e) => {
                        tracing::error!(?e, "{device:#?}");
                        None
                    }
                })
                .collect());
        }
    }
}
