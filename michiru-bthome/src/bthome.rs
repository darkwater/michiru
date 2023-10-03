use anyhow::{bail, Result};
use bytes::Buf;
use michiru_device::{DataType, Format, Payload, PropertyAttributes, Unit};

#[derive(Debug, PartialEq)]
pub enum Object {
    Battery(f32),
    Temperature(f32),
    Humidity(f32),
    Voltage(f32),
    Power(bool),
}

impl Object {
    pub fn decode(mut data: impl Buf) -> Result<Vec<Self>> {
        data.copy_to_bytes(3);

        let mut out = vec![];

        while data.has_remaining() {
            let header = data.get_u8();
            let len = header & 0b11111;
            let ty = header >> 5;
            // println!("len: {}, ty: {}", len, ty);

            let mut data = data.copy_to_bytes(len as usize);
            // println!("{:#02x?}", &data[..]);

            let object_id = data.get_u8();
            let value = match (len, ty) {
                (2, 0) => data.get_u8() as f32,
                (3, 0) => data.get_u16_le() as f32,
                (2, 1) => data.get_i8() as f32,
                (3, 1) => data.get_i16_le() as f32,
                (5, 2) => data.get_f32_le(),
                _ => bail!("unimplemented object type"),
            };

            let obj = match object_id {
                0x01 => Self::Battery(value),
                0x02 => Self::Temperature(value * 0.01),
                0x03 => Self::Humidity(value * 0.01),
                0x0c => Self::Voltage(value * 0.001),
                0x10 => Self::Power(value > 0.),
                _ => bail!("unimplemented object id"),
            };

            out.push(obj);
        }

        Ok(out)
    }

    pub fn into_michiru(self) -> (PropertyAttributes, Payload) {
        match self {
            Object::Battery(v) => (
                PropertyAttributes {
                    id: "battery".into(),
                    name: "Battery".into(),
                    datatype: DataType::Float,
                    settable: false,
                    retained: true,
                    unit: Some(Unit::Percent),
                    format: Some(Format::FloatRange(0., 100.)),
                },
                Payload::Float(v as f64),
            ),
            Object::Temperature(v) => (
                PropertyAttributes {
                    id: "temperature".into(),
                    name: "Temperature".into(),
                    datatype: DataType::Float,
                    settable: false,
                    retained: true,
                    unit: Some(Unit::DegreeCelsius),
                    format: None,
                },
                Payload::Float(v as f64),
            ),
            Object::Humidity(v) => (
                PropertyAttributes {
                    id: "humidity".into(),
                    name: "Humidity".into(),
                    datatype: DataType::Float,
                    settable: false,
                    retained: true,
                    unit: Some(Unit::Percent),
                    format: Some(Format::FloatRange(0., 100.)),
                },
                Payload::Float(v as f64),
            ),
            Object::Voltage(v) => (
                PropertyAttributes {
                    id: "voltage".into(),
                    name: "Battery voltage".into(),
                    datatype: DataType::Float,
                    settable: false,
                    retained: true,
                    unit: Some(Unit::Volts),
                    format: None,
                },
                Payload::Float(v as f64),
            ),
            Object::Power(v) => (
                PropertyAttributes {
                    id: "power".into(),
                    name: "Power".into(),
                    datatype: DataType::Boolean,
                    settable: false,
                    retained: true,
                    unit: None,
                    format: None,
                },
                Payload::Boolean(v),
            ),
        }
    }
}
