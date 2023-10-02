use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local};
use itertools::Itertools;
use rumqttc::{LastWill, QoS};

pub use rumqttc::MqttOptions;

pub const BASE_TOPIC: &str = "homie";
pub const QOS: QoS = QoS::AtLeastOnce;
pub const HOMIE_VERSION: &str = "4.0.0";

#[derive(Debug, Clone, Copy)]
pub enum DataType {
    Integer,
    Float,
    Boolean,
    String,
    Enum,
    Color,
}

impl From<DataType> for Vec<u8> {
    fn from(val: DataType) -> Self {
        match val {
            DataType::Integer => b"integer".to_vec(),
            DataType::Float => b"float".to_vec(),
            DataType::Boolean => b"boolean".to_vec(),
            DataType::String => b"string".to_vec(),
            DataType::Enum => b"enum".to_vec(),
            DataType::Color => b"color".to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Payload {
    String(String),
    Integer(i64),
    Float(f64),
    Percent(f64),
    Boolean(bool),
    Enum(String),
    Color(Color),
    DateTime(DateTime<Local>),
    Duration(Duration),
}

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Rgb(u8, u8, u8),
    Hsv(u16, u8, u8),
}

#[derive(Debug, Clone)]
pub struct DeviceAttributes {
    pub id: String,
    pub homie: String,
    pub name: String,
    pub state: DeviceState,
    pub nodes: Vec<NodeAttributes>,
    pub extensions: Vec<String>,
    pub implementation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeAttributes {
    pub id: String,
    pub name: String,
    pub type_: String,
    pub properties: Vec<PropertyAttributes>,
}

#[derive(Debug, Clone)]
pub struct PropertyAttributes {
    pub id: String,
    pub name: String,
    pub datatype: DataType,
    pub settable: bool,
    pub retained: bool,
    pub unit: Option<Unit>,
    pub format: Option<Format>,
}

// depends on DataType, maybe don't put in one enum like this?
#[derive(Debug, Clone)]
pub enum Format {
    IntRange(i64, i64),
    FloatRange(f64, f64),
    Enum(Vec<String>),
    ColorRgb,
    ColorHsv,
}

impl From<Format> for Vec<u8> {
    fn from(format: Format) -> Self {
        match format {
            Format::IntRange(a, b) => format!("{}:{}", a, b).into_bytes(),
            Format::FloatRange(a, b) => format!("{}:{}", a, b).into_bytes(),
            Format::Enum(values) => values.join(",").into_bytes(),
            Format::ColorRgb => b"rgb".to_vec(),
            Format::ColorHsv => b"hsv".to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Unit {
    DegreeCelsius,
    DegreeFahrenheit,
    Degree,
    Liter,
    Galon,
    Volts,
    Watt,
    Ampere,
    Percent,
    Meter,
    Feet,
    Pascal,
    Psi,
    Count,
    Other(String),
}

impl From<Unit> for Vec<u8> {
    fn from(unit: Unit) -> Self {
        match unit {
            Unit::DegreeCelsius => "°C".to_string().into_bytes(),
            Unit::DegreeFahrenheit => "°F".to_string().into_bytes(),
            Unit::Degree => "°".to_string().into_bytes(),
            Unit::Liter => b"L".to_vec(),
            Unit::Galon => b"gal".to_vec(),
            Unit::Volts => b"V".to_vec(),
            Unit::Watt => b"W".to_vec(),
            Unit::Ampere => b"A".to_vec(),
            Unit::Percent => b"%".to_vec(),
            Unit::Meter => b"m".to_vec(),
            Unit::Feet => b"ft".to_vec(),
            Unit::Pascal => b"Pa".to_vec(),
            Unit::Psi => b"psi".to_vec(),
            Unit::Count => b"#".to_vec(),
            Unit::Other(v) => v.into_bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceState {
    Init,
    Ready,
    Disconnected,
    Sleeping,
    Lost,
    Alert,
}

impl From<DeviceState> for Vec<u8> {
    fn from(val: DeviceState) -> Self {
        match val {
            DeviceState::Init => b"init".to_vec(),
            DeviceState::Ready => b"ready".to_vec(),
            DeviceState::Disconnected => b"disconnected".to_vec(),
            DeviceState::Sleeping => b"sleeping".to_vec(),
            DeviceState::Lost => b"lost".to_vec(),
            DeviceState::Alert => b"alert".to_vec(),
        }
    }
}

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
    attributes: NodeAttributes,
    properties: Vec<PropertyAttributes>,
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
        let id = node.id.clone();
        self.device.nodes.push(node);
        let node = self.device.node(&id).unwrap();

        node.send_topic("$name", node.attributes.name.clone())
            .await?;
        node.send_topic("$type", node.attributes.type_.clone())
            .await?;
        node.send_topic(
            "$properties",
            node.attributes
                .properties
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<&str>>()
                .join(","),
        )
        .await?;

        for property in node.properties() {
            property
                .send_topic("$name", property.attributes.name.clone())
                .await?;

            property
                .send_topic("$datatype", property.attributes.datatype)
                .await?;

            property
                .send_topic("$settable", property.attributes.settable.to_string())
                .await?;

            property
                .send_topic("$retained", property.attributes.retained.to_string())
                .await?;

            if let Some(format) = &property.attributes.format {
                property.send_topic("$format", format.clone()).await?;
            }

            if let Some(unit) = &property.attributes.unit {
                property.send_topic("$unit", unit.clone()).await?;
            }
        }

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
                attributes: attributes.clone(),
                properties: attributes.properties.clone(),
            })
    }

    pub fn nodes(&self) -> Vec<Node> {
        self.nodes
            .iter()
            .map(|attributes| Node {
                device: self,
                attributes: attributes.clone(),
                properties: attributes.properties.clone(),
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

    pub fn property(&self, id: &str) -> Option<Property> {
        self.properties
            .iter()
            .find(|property| property.id == id)
            .map(|attributes| Property {
                node: self,
                attributes: attributes.clone(),
            })
    }

    pub fn properties(&self) -> Vec<Property> {
        self.properties
            .iter()
            .map(|attributes| Property {
                node: self,
                attributes: attributes.clone(),
            })
            .collect()
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
            Payload::Duration(v) => v.num_seconds().to_string().into_bytes(),
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
