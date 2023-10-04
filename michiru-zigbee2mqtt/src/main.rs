mod definitions;

use std::time::Duration;

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};

use crate::definitions::DeviceInfo;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut mqttoptions = MqttOptions::new("michiru", "michiru.fbk.red", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_max_packet_size(1024 * 1024, 1024 * 1024);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client
        .subscribe("zigbee2mqtt/bridge/devices", QoS::ExactlyOnce)
        .await
        .unwrap();

    loop {
        let notification = eventloop.poll().await.unwrap();
        tracing::trace!(?notification);

        if let Event::Incoming(Packet::Publish(obj)) = notification {
            let Ok(devices) = serde_json::from_slice::<Vec<serde_json::Value>>(&obj.payload) else {
                continue;
            };

            for device in devices {
                match serde_json::from_value::<DeviceInfo>(device.clone()) {
                    Ok(devices) => {
                        tracing::info!("{devices:#?}");
                    }
                    Err(e) => {
                        tracing::error!(?e, ?device);
                    }
                }
            }
        }
    }
}
