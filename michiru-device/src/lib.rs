mod attributes;
mod payload;
mod utils;

use anyhow::{Context, Result};
use itertools::Itertools;
use rumqttc::{LastWill, QoS};

pub use rumqttc::MqttOptions;

pub use attributes::*;
pub use payload::*;

pub const BASE_TOPIC: &str = "homie";
pub const QOS: QoS = QoS::AtLeastOnce;
pub const HOMIE_VERSION: &str = "4.0.0";

#[must_use]
pub struct DeviceBuilder {
    device: Device,
}

pub struct Device {
    mqtt: rumqttc::AsyncClient,
    id: String,
    nodes: Vec<NodeAttributes>,
}

pub struct Node<'a> {
    device: &'a Device,
    attributes: &'a NodeAttributes,
}

pub struct Property<'a> {
    node: &'a Node<'a>,
    attributes: PropertyAttributes,
}

impl DeviceBuilder {
    /// Will override any last will on the options
    pub async fn new(
        mut options: MqttOptions,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self> {
        let id = id.into();
        let name = name.into();

        if !utils::valid_topic_id(&id) {
            return Err(anyhow::anyhow!("Invalid device id"));
        }

        options.set_last_will(LastWill::new(
            format!("{BASE_TOPIC}/{id}/$state"),
            DeviceState::Lost,
            QOS,
            true,
        ));

        let (mqtt, mut connection) = rumqttc::AsyncClient::new(options, 10);

        tokio::spawn({
            let id = id.clone();
            async move {
                loop {
                    let event = connection.poll().await.unwrap();
                    tracing::trace!(?id, "Event = {:?}", event);
                }
            }
        });

        let device = Device {
            mqtt,
            id,
            nodes: vec![],
        };

        device.send_topic("$homie", HOMIE_VERSION).await?;
        device.send_topic("$state", DeviceState::Init).await?;
        device.send_topic("$name", name).await?;

        Ok(Self { device })
    }

    pub async fn node(mut self, node: NodeAttributes) -> Result<Self> {
        if !utils::valid_topic_id(&node.id) {
            return Err(anyhow::anyhow!("Invalid node id"));
        }

        let id = node.id.clone();
        self.device.nodes.push(node);
        let node = self.device.node(&id).unwrap();

        node.advertise().await?;

        Ok(self)
    }

    pub async fn build(self) -> Result<Device> {
        self.device
            .send_topic("$nodes", self.device.nodes.iter().map(|n| &n.id).join(","))
            .await?;

        self.device.send_topic("$state", DeviceState::Ready).await?;

        Ok(self.device)
    }
}

impl Device {
    async fn send_topic(&self, topic: &str, payload: impl Into<Vec<u8>>) -> Result<()> {
        self.send_topic_with_retain(topic, payload, true).await
    }

    async fn send_topic_with_retain(
        &self,
        topic: &str,
        payload: impl Into<Vec<u8>>,
        retain: bool,
    ) -> Result<()> {
        self.mqtt
            .publish(
                format!("{BASE_TOPIC}/{id}/{topic}", id = self.id),
                QOS,
                retain,
                payload,
            )
            .await
            .with_context(|| format!("Failed to publish to topic {topic}"))
    }

    pub fn node(&self, id: &str) -> Option<Node> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|attributes| Node {
                device: self,
                attributes: attributes,
            })
    }

    pub fn nodes(&self) -> Vec<Node> {
        self.nodes
            .iter()
            .map(|attributes| Node {
                device: self,
                attributes: attributes,
            })
            .collect()
    }

    pub async fn disconnect(self) -> Result<()> {
        self.send_topic("$state", DeviceState::Disconnected).await?;
        self.mqtt.disconnect().await.context("Failed to disconnect")
    }
}

impl Node<'_> {
    async fn send_topic(&self, topic: &str, payload: impl Into<Vec<u8>>) -> Result<()> {
        self.send_topic_with_retain(topic, payload, true).await
    }

    async fn send_topic_with_retain(
        &self,
        topic: &str,
        payload: impl Into<Vec<u8>>,
        retain: bool,
    ) -> Result<()> {
        self.device
            .send_topic_with_retain(
                format!("{}/{}", self.attributes.id, topic).as_str(),
                payload,
                retain,
            )
            .await
    }

    async fn advertise(&self) -> Result<()> {
        self.send_topic("$name", self.attributes.name.clone())
            .await?;
        self.send_topic("$type", self.attributes.type_.clone())
            .await?;
        self.send_topic(
            "$properties",
            self.attributes
                .properties
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<&str>>()
                .join(","),
        )
        .await?;

        for property in self.properties() {
            property.advertise().await?;
        }

        Ok(())
    }

    pub fn property(&self, id: &str) -> Option<Property> {
        self.attributes
            .properties
            .iter()
            .find(|property| property.id == id)
            .map(|attributes| Property {
                node: self,
                attributes: attributes.clone(),
            })
    }

    pub fn properties(&self) -> Vec<Property> {
        self.attributes
            .properties
            .iter()
            .map(|attributes| Property {
                node: self,
                attributes: attributes.clone(),
            })
            .collect()
    }

    pub async fn property_or_insert(&mut self, attributes: PropertyAttributes) -> Result<Property> {
        // work around a limitiation of borrowck??
        if self.property(&attributes.id).is_some() {
            return Ok(self.property(&attributes.id).unwrap());
        }

        self.device.send_topic("$state", DeviceState::Init).await?;

        let id = attributes.id.clone();

        // TODO: make this possible
        // self.attributes.properties.push(attributes);

        self.advertise().await?;

        self.device.send_topic("$state", DeviceState::Ready).await?;

        Ok(self.property(&id).unwrap())
    }
}

impl Property<'_> {
    async fn send_topic(&self, topic: &str, payload: impl Into<Vec<u8>>) -> Result<()> {
        self.node
            .send_topic(
                format!("{}/{}", self.attributes.id, topic).as_str(),
                payload,
            )
            .await
    }

    async fn advertise(&self) -> Result<()> {
        self.send_topic("$name", self.attributes.name.clone())
            .await?;

        self.send_topic("$datatype", self.attributes.datatype)
            .await?;

        self.send_topic("$settable", self.attributes.settable.to_string())
            .await?;

        self.send_topic("$retained", self.attributes.retained.to_string())
            .await?;

        if let Some(format) = &self.attributes.format {
            self.send_topic("$format", format.clone()).await?;
        }

        if let Some(unit) = &self.attributes.unit {
            self.send_topic("$unit", unit.clone()).await?;
        }

        Ok(())
    }

    pub async fn send(&self, payload: Payload) -> Result<()> {
        let payload = match payload {
            Payload::String(v) => v.into_bytes(),
            Payload::Integer(v) => v.to_string().into_bytes(),
            Payload::Float(v) => v.to_string().into_bytes(),
            Payload::Percent(v) => v.to_string().into_bytes(),
            Payload::Boolean(v) => v.to_string().into_bytes(),
            Payload::Enum(v) => v.into_bytes(),
            Payload::Color(v) => match v {
                Color::Rgb(r, g, b) => format!("{},{},{}", r, g, b).into_bytes(),
                Color::Hsv(h, s, v) => format!("{},{},{}", h, s, v).into_bytes(),
            },
            Payload::DateTime(v) => v.to_rfc3339().into_bytes(),
            Payload::Duration(_) => todo!(),
        };

        self.node
            .send_topic_with_retain(
                self.attributes.id.as_str(),
                payload,
                self.attributes.retained,
            )
            .await
    }
}
