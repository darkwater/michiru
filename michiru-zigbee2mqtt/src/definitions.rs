use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub ieee_address: String,
    pub model_id: String,
    pub manufacturer: String,
    pub power_source: String,
    #[serde(rename = "type")]
    pub zigbee_device_type: ZigbeeDeviceType,
    pub interview_completed: bool,
    pub disabled: bool,
    pub definition: DeviceDefinition,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ZigbeeDeviceType {
    Coordinator,
    Router,
    EndDevice,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceDefinition {
    pub description: String,
    pub exposes: Vec<DeviceDefinitionExpose>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceDefinitionExpose {
    pub access: DeviceDefinitionExposeAccess,
    #[serde(flatten)]
    pub expose_type: DeviceDefinitionExposeType,
    pub name: Option<String>,
    pub property: Option<String>,
    pub unit: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct DeviceDefinitionExposeAccess {
    pub published: bool,
    pub settable: bool,
    pub gettable: bool,
}

impl<'de> Deserialize<'de> for DeviceDefinitionExposeAccess {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let access = u32::deserialize(deserializer)?;

        Ok(Self {
            published: access & 1 != 0,
            settable: access & 2 != 0,
            gettable: access & 4 != 0,
        })
    }
}

impl Serialize for DeviceDefinitionExposeAccess {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut access = 0;

        if self.published {
            access |= 1;
        }

        if self.settable {
            access |= 2;
        }

        if self.gettable {
            access |= 4;
        }

        access.serialize(serializer)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum DeviceDefinitionExposeType {
    Numeric, // TODO: value_min, value_max
    Enum {
        values: Vec<String>,
    },
    Switch,
    Binary {
        value_on: Value,
        value_off: Value,
        value_toggle: Option<Value>,
    },
}
