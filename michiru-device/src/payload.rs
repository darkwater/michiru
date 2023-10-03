use chrono::{DateTime, Duration, Local};

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
