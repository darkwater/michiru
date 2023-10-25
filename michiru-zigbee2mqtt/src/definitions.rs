use std::fmt;

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
pub struct DeviceDefinition {
    pub description: String,
    pub exposes: Vec<Expose>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum Expose {
    Generic(Feature),
    Specific(SpecificFeature),
}

impl Expose {
    pub fn ty(&self) -> FeatureType {
        match self {
            Expose::Generic(f) => f.ty(),
            Expose::Specific(s) => s.ty,
        }
    }

    pub fn property(&self) -> &str {
        match self {
            Expose::Generic(f) => f.meta().property.as_str(),
            Expose::Specific(s) => s.features[0].meta().property.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Expose::Generic(f) => f.meta().name.as_str(),
            // note: specific features don't have a name
            Expose::Specific(s) => s.features[0].meta().property.as_str(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum Feature {
    Binary {
        #[serde(flatten)]
        meta: FeatureMeta,
        value_on: Value,
        value_off: Value,
        value_toggle: Option<Value>,
    },
    Numeric {
        #[serde(flatten)]
        meta: FeatureMeta,
        value_min: Option<f64>,
        value_max: Option<f64>,
        value_step: Option<f64>,
        #[serde(default)]
        unit: Option<String>,
        #[serde(default)]
        presets: Vec<Preset>,
    },
    Text {
        #[serde(flatten)]
        meta: FeatureMeta,
    },
    Enum {
        #[serde(flatten)]
        meta: FeatureMeta,
        values: Vec<String>,
    },
    Composite {
        #[serde(flatten)]
        meta: FeatureMeta,
        features: Vec<Feature>,
    },
    List,
}

impl Feature {
    pub fn ty(&self) -> FeatureType {
        match self {
            Feature::Binary { .. } => FeatureType::Binary,
            Feature::Numeric { .. } => FeatureType::Numeric,
            Feature::Text { .. } => FeatureType::Text,
            Feature::Enum { .. } => FeatureType::Enum,
            Feature::Composite { .. } => FeatureType::Composite,
            Feature::List => FeatureType::List,
        }
    }

    pub fn meta(&self) -> &FeatureMeta {
        match self {
            Feature::Binary { meta, .. } => meta,
            Feature::Numeric { meta, .. } => meta,
            Feature::Text { meta, .. } => meta,
            Feature::Enum { meta, .. } => meta,
            Feature::Composite { meta, .. } => meta,
            Feature::List => unimplemented!(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FeatureMeta {
    pub access: FeatureAccess,
    pub name: String,
    pub property: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub struct SpecificFeature {
    pub features: Vec<Feature>,
    #[serde(rename = "type")]
    pub ty: FeatureType,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureType {
    Binary,
    Numeric,
    Text,
    Enum,
    Composite,
    List,
    Light,
    Switch,
    Fan,
    Cover,
    Lock,
    Climate,
}

impl fmt::Display for FeatureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Preset {
    name: String,
    value: f64,
    description: Option<String>,
}

#[derive(Debug)]
pub struct FeatureAccess {
    pub published: bool,
    pub settable: bool,
    pub gettable: bool,
}

impl<'de> Deserialize<'de> for FeatureAccess {
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

impl Serialize for FeatureAccess {
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
pub enum ZigbeeDeviceType {
    Coordinator,
    Router,
    EndDevice,
}

impl fmt::Display for ZigbeeDeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZigbeeDeviceType::Coordinator => write!(f, "Coordinator"),
            ZigbeeDeviceType::Router => write!(f, "Router"),
            ZigbeeDeviceType::EndDevice => write!(f, "End Device"),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserialize_exposes() {
        let res = serde_json::from_value::<Expose>(json!(
            {
                "access": 1,
                "description": "Triggered action (e.g. a button click)",
                "label": "Action",
                "name": "action",
                "property": "action",
                "type": "enum",
                "values": [
                    "on",
                    "off",
                    "brightness_move_up",
                    "brightness_stop"
                ]
            }
        ));

        assert!(res.is_ok(), "{:#?}", res);
        let res = res.unwrap();

        let res = serde_json::from_value::<Expose>(json!(
            {
                "access": 1,
                "description": "Link quality (signal strength)",
                "label": "Linkquality",
                "name": "linkquality",
                "property": "linkquality",
                "type": "numeric",
                "unit": "lqi",
                "value_max": 255,
                "value_min": 0
            }
        ));

        assert!(res.is_ok(), "{:#?}", res);
        let res = res.unwrap();
    }
}
