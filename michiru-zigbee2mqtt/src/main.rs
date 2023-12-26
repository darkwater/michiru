mod definitions;
mod devices;

use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use michiru_device::{
    DataType, DeviceBuilder, Format, MqttOptions, NodeAttributes, PropertyAttributes, Unit,
};
use michiru_zigbee2mqtt::{
    definitions::{DeviceDefinition, DeviceInfo, Expose, Feature, FeatureType},
    DefinitionStream,
};
use rumqttc::{AsyncClient, Event, Packet, QoS};
use serde::Deserialize;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut mqttoptions = MqttOptions::new("michiru-zigbee2mqtt", "michiru.fbk.red", 1883);
    mqttoptions.set_max_packet_size(16 * 1024 * 1024, 1024);

    let mut stream = DefinitionStream::new(mqttoptions).await;

    let mut handles = Vec::<JoinHandle<()>>::new();

    loop {
        let devices = stream.next().await.unwrap();

        handles.drain(..).for_each(|h| h.abort());

        for device in devices {
            handles.push(tokio::spawn(async move {
                handle_device(device).await.unwrap();
            }));
        }
    }
}

async fn handle_device(device: DeviceInfo) -> Result<()> {
    let id = format!("zigbee2mqtt-{}", device.ieee_address);
    let name = device.model_id;
    let options = MqttOptions::new(format!("michiru-{}", id), "michiru.fbk.red", 1883);
    let mut homie = DeviceBuilder::new(options, id.clone(), name.clone())
        .await?
        .node(NodeAttributes {
            id: "link".to_string(),
            name: "Link".to_string(),
            type_: "Zigbee".to_string(),
            properties: vec![PropertyAttributes {
                id: "quality".to_string(),
                name: "Quality".to_string(),
                datatype: DataType::Integer,
                settable: false,
                retained: true,
                unit: Some(Unit::Other("lqi".to_string())),
                format: Some(Format::IntRange(0, 255)),
            }],
        })
        .await?;

    let options = MqttOptions::new(format!("michiru-{}-listener", id), "michiru.fbk.red", 1883);
    let (listener_client, mut listener) = AsyncClient::new(options, 10);
    listener_client
        .subscribe(format!("zigbee2mqtt/{}", device.friendly_name), QoS::ExactlyOnce)
        .await
        .unwrap();

    match name.as_str() {
        "TRADFRI SHORTCUT Button" => {
            let device = homie
                .node(NodeAttributes {
                    id: "battery".to_string(),
                    name: "Battery".to_string(),
                    type_: "CR2032".to_string(),
                    properties: vec![PropertyAttributes {
                        id: "level".to_string(),
                        name: "Level".to_string(),
                        datatype: DataType::Integer,
                        settable: false,
                        retained: true,
                        unit: Some(Unit::Percent),
                        format: Some(Format::IntRange(0, 100)),
                    }],
                })
                .await?
                .node(NodeAttributes {
                    id: "input".to_string(),
                    name: "Input".to_string(),
                    type_: "Push button".to_string(),
                    properties: vec![PropertyAttributes {
                        id: "action".to_string(),
                        name: "Action".to_string(),
                        datatype: DataType::Enum,
                        settable: false,
                        retained: true,
                        unit: Some(Unit::Percent),
                        format: Some(Format::Enum(match device.definition.property("action") {
                            Some(Feature::Enum { values, .. }) => values.clone(),
                            _ => unreachable!("Invalid enum definition"),
                        })),
                    }],
                })
                .await?
                .build()
                .await
                .unwrap();

            #[derive(Debug, Deserialize)]
            struct Data {
                battery: u8,
                linkquality: u8,
                action: String,
            }

            while let Ok(event) = listener.poll().await {
                let Event::Incoming(Packet::Publish(obj)) = event else {
                    continue;
                };

                tracing::info!("{:#?}", obj.payload);
            }
        }
        _ => tracing::warn!("Unsupported device: {}", name),
    }

    // for expose in device.definition.exposes {
    //     match (&expose, expose.property()) {
    //         (Expose::Generic(Feature::Numeric { .. }), "linkquality") => {
    //             homie = homie
    //                 .node(NodeAttributes {
    //                     id: "link".to_string(),
    //                     name: "Link".to_string(),
    //                     type_: "Zigbee".to_string(),
    //                     properties: vec![PropertyAttributes {
    //                         id: "quality".to_string(),
    //                         name: "Quality".to_string(),
    //                         datatype: DataType::Integer,
    //                         settable: false,
    //                         retained: true,
    //                         unit: Some(Unit::Other("lqi".to_string())),
    //                         format: Some(Format::IntRange(0, 255)),
    //                     }],
    //                 })
    //                 .await?;
    //         }
    //         (Expose::Generic(Feature::Numeric { .. }), "battery") => {
    //             homie = homie
    //                 .node(NodeAttributes {
    //                     id: "battery".to_string(),
    //                     name: "Battery".to_string(),
    //                     type_: "Battery".to_string(),
    //                     properties: vec![PropertyAttributes {
    //                         id: "level".to_string(),
    //                         name: "Level".to_string(),
    //                         datatype: DataType::Integer,
    //                         settable: false,
    //                         retained: true,
    //                         unit: Some(Unit::Other("lqi".to_string())),
    //                         format: Some(Format::IntRange(0, 100)),
    //                     }],
    //                 })
    //                 .await?;
    //         }
    //         (Expose::Generic(Feature::Enum { values, .. }), "action") => {
    //             homie = homie
    //                 .node(NodeAttributes {
    //                     id: "action".to_string(),
    //                     name: "Action".to_string(),
    //                     type_: "Action".to_string(),
    //                     properties: vec![PropertyAttributes {
    //                         id: "action".to_string(),
    //                         name: "Action".to_string(),
    //                         datatype: DataType::Enum,
    //                         settable: false,
    //                         retained: false,
    //                         unit: None,
    //                         format: Some(Format::Enum(values.clone())),
    //                     }],
    //                 })
    //                 .await?;
    //         }
    //         other => {
    //             tracing::error!("Unsupported expose: {:?}", other);
    //         }
    //     }
    // }

    Ok(())
}
