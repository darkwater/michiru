use std::collections::hash_map::Entry;
use std::collections::HashMap;

use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::StreamExt;
use michiru_device::{
    DataType, DeviceBuilder, MqttOptions, NodeAttributes, Payload, PropertyAttributes, Unit,
};

use crate::bthome::Object;

mod bthome;

#[derive(Debug, PartialEq)]
pub struct Update {
    name: String,
    object: Object,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let manager = Manager::new().await?;

    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().expect("no adapters found");

    let mut events = central.events().await?;

    central.start_scan(ScanFilter::default()).await?;

    let mut devices = HashMap::new();

    while let Some(event) = events.next().await {
        if let CentralEvent::ServiceDataAdvertisement { id, service_data } = event {
            if let Some(data) = service_data.get(&uuid_from_u16(0x181c)) {
                let peripherals = central.peripherals().await?;

                let Some(peripheral) = peripherals.iter().find(|p| p.id() == id) else {
                    eprintln!("got ad from unknown peripheral");
                    continue;
                };

                let Some(properties) = peripheral.properties().await? else {
                    eprintln!("got ad from peripheral with no properties");
                    continue;
                };

                let Some(name) = properties.local_name else {
                    eprintln!("got ad from peripheral with no name");
                    continue;
                };

                #[cfg(not(target_os = "macos"))]
                let id = properties.address.to_string_no_delim().to_lowercase();
                #[cfg(target_os = "macos")]
                let id = name.to_lowercase();

                let id = format!("bthome-{id}");

                const LINK_ID: &str = "link";
                const RSSI_ID: &str = "rssi";
                const SENSOR_ID: &str = "sensor";

                let device = match devices.entry(id.clone()) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert({
                        let options = MqttOptions::new(id.clone(), "michiru.fbk.red", 1883);

                        DeviceBuilder::new(options, id, name.clone())
                            .await?
                            .node(NodeAttributes {
                                id: LINK_ID.into(),
                                name: "Link".into(),
                                type_: "Link".into(),
                                properties: vec![PropertyAttributes {
                                    id: RSSI_ID.into(),
                                    name: "RSSI".into(),
                                    datatype: DataType::Integer,
                                    settable: false,
                                    retained: true,
                                    unit: Some(Unit::Other("dBm".into())),
                                    format: None,
                                }],
                            })
                            .await?
                            .node(NodeAttributes {
                                id: SENSOR_ID.into(),
                                name: "Sensor".into(),
                                type_: "Sensor".into(),
                                properties: vec![],
                            })
                            .await?
                            .build()
                            .await?
                    }),
                };

                if let Some(rssi) = properties.rssi {
                    device
                        .node(LINK_ID)
                        .unwrap()
                        .property(RSSI_ID)
                        .unwrap()
                        .send(Payload::Integer(rssi as i64))
                        .await?;
                }

                for object in Object::decode(data.as_slice())? {
                    let (attrs, payload) = object.into_michiru();

                    device
                        .node(SENSOR_ID)
                        .unwrap()
                        .property_or_insert(attrs)
                        .await?
                        .send(payload)
                        .await?;
                }
            }
        }
    }

    Ok(())
}
