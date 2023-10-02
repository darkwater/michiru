use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::StreamExt;
use michiru_device::{
    DataType, DeviceBuilder, MqttOptions, NodeAttributes, PropertyAttributes, Unit,
};

use crate::bthome::Object;

mod bthome;
mod device;

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

    let options = MqttOptions::new("michiru-bthome", "192.168.0.106", 1883);
    let device = DeviceBuilder::new(options, "bthome-123", "BTHome sensor")
        .await?
        .node(NodeAttributes {
            id: "sensor".into(),
            name: "Sensor".into(),
            type_: "Sensor".into(),
            properties: vec![PropertyAttributes {
                id: "temperature".into(),
                name: "Temperature".into(),
                datatype: DataType::Float,
                settable: false,
                retained: true,
                unit: Some(Unit::DegreeCelsius),
                format: None,
            }],
        })
        .await?
        .build();

    while let Some(event) = events.next().await {
        if let CentralEvent::ServiceDataAdvertisement { id, service_data } = event {
            if let Some(data) = service_data.get(&uuid_from_u16(0x181c)) {
                let peripherals = central.peripherals().await.unwrap();

                let Some(peripheral) = peripherals.iter().find(|p| p.id() == id) else {
                    eprintln!("got ad from unknown peripheral");
                    continue;
                };

                let Some(properties) = peripheral.properties().await.unwrap() else {
                    eprintln!("got ad from peripheral with no properties");
                    continue;
                };

                let Some(name) = properties.local_name else {
                    eprintln!("got ad from peripheral with no name");
                    continue;
                };

                let objects = match bthome::decode(data.as_slice()).await {
                    Ok(objects) => objects,
                    Err(e) => {
                        eprintln!("failed to decode ad: {}", e);
                        continue;
                    }
                };

                // if let Some(rssi) = properties.rssi {
                //     objects.push(Update {
                //         name: name.clone(),
                //         object: Object::Rssi(rssi),
                //     });
                // }

                println!("{name} {objects:?}");
            }
        }
    }

    Ok(())
}
