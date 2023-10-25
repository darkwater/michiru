mod definitions;

use std::time::Duration;

use itertools::Itertools;
use rumqttc::{AsyncClient, ConnectionError, Event, MqttOptions, Packet, QoS};
use rxrust::prelude::*;
use tokio::task::JoinHandle;

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

    let mut handle = None::<JoinHandle<()>>;

    loop {
        let notification = eventloop.poll().await.unwrap();
        tracing::trace!(?notification);

        let Event::Incoming(Packet::Publish(obj)) = notification else {
            continue;
        };

        let Ok(devices) = serde_json::from_slice::<Vec<serde_json::Value>>(&obj.payload) else {
            continue;
        };

        let devices = devices
            .iter()
            .filter_map(
                |device| match serde_json::from_value::<DeviceInfo>(device.clone()) {
                    Ok(device) => {
                        // tracing::info!("{devices:#?}");
                        Some(device)
                    }
                    Err(e) => {
                        tracing::error!(?e, "{device:#?}");
                        None
                    }
                },
            )
            .collect_vec();

        if let Some(handle) = handle.take() {
            handle.abort();
        }

        let mut n = 0;
        handle = Some(tokio::spawn(async move {
            // let mut devices = devices.into_iter().map(|device| device.id).collect_vec();
            // devices.sort();
            // tracing::info!(?devices);
            println!("{}", devices.len());
            loop {
                println!("{n}");
                n += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));
    }
}
