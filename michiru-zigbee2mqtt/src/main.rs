mod definitions;

use std::time::Duration;

use anyhow::Result;
use michiru_device::{
    DataType, DeviceBuilder, Format, MqttOptions, NodeAttributes, PropertyAttributes, Unit,
};
use michiru_zigbee2mqtt::{
    definitions::{DeviceDefinition, DeviceInfo, Expose, Feature, FeatureType},
    DefinitionStream,
};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut mqttoptions = MqttOptions::new("michiru-zigbee2mqtt", "michiru.fbk.red", 1883);
    mqttoptions.set_max_packet_size(16 * 1024 * 1024, 1024);

    let mut stream = DefinitionStream::new(mqttoptions).await;

    let mut handle = None::<JoinHandle<()>>;

    loop {
        let devices = stream.next().await.unwrap();

        if let Some(handle) = handle.take() {
            handle.abort();
        }

        let mut n = 0;
        handle = Some(tokio::spawn(async move {
            for device in devices {
                handle_device(device);
            }
        }));
    }
}

async fn handle_device(device: DeviceInfo) -> Result<()> {
    let options = MqttOptions::new("michiru-zigbee2mqtt-{}", "michiru.fbk.red", 1883);
    let id = format!("zigbee2mqtt-{}", device.ieee_address);
    let name = device.model_id;

    let mut homie = DeviceBuilder::new(options, id, name).await?;
    for expose in device.definition.exposes {
        match (&expose, expose.property()) {
            (Expose::Generic(Feature::Numeric { .. }), "linkquality") => {
                homie = homie
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
            }
            (Expose::Generic(Feature::Numeric { .. }), "battery") => {
                homie = homie
                    .node(NodeAttributes {
                        id: "battery".to_string(),
                        name: "Battery".to_string(),
                        type_: "Battery".to_string(),
                        properties: vec![PropertyAttributes {
                            id: "level".to_string(),
                            name: "Level".to_string(),
                            datatype: DataType::Integer,
                            settable: false,
                            retained: true,
                            unit: Some(Unit::Other("lqi".to_string())),
                            format: Some(Format::IntRange(0, 100)),
                        }],
                    })
                    .await?;
            }
            (Expose::Generic(Feature::Enum { values, .. }), "action") => {
                homie = homie
                    .node(NodeAttributes {
                        id: "action".to_string(),
                        name: "Action".to_string(),
                        type_: "Action".to_string(),
                        properties: vec![PropertyAttributes {
                            id: "action".to_string(),
                            name: "Action".to_string(),
                            datatype: DataType::Enum,
                            settable: false,
                            retained: false,
                            unit: None,
                            format: Some(Format::Enum(values.clone())),
                        }],
                    })
                    .await?;
            }
            _ => todo!(),
        }
    }

    Ok(())
}
