use crate::payload::DataType;

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
