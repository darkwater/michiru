use std::{io::ErrorKind, time::Duration};

use anyhow::{Context, Result};
use nom::{number::complete::u8, IResult};
use serialport::{SerialPortType, UsbPortInfo};

fn main() -> Result<()> {
    let port = serialport::available_ports()
        .context("No serial ports found")?
        .into_iter()
        .find(|p| match p.port_type {
            SerialPortType::UsbPort(UsbPortInfo { product: Some(ref product), .. }) => {
                product.contains("CC2531")
            }
            _ => false,
        })
        .context("No CC2531 found")?;

    let mut port = serialport::new(port.port_name, 115200)
        .open()
        .context("Failed to open serial port")?;

    port.set_timeout(Duration::from_secs(10))
        .context("Failed to set timeout")?;

    port.write_all(&[0xfe, 0x00, 0x21, 0x01, 0x20])?;

    let mut buf = [0u8; 1024];
    loop {
        let n = match port.read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::TimedOut => continue,
            Err(e) => return Err(e).context("Failed to read from serial port"),
        };

        if n > 0 {
            let msg = Message::parse(&buf[..n]);
            println!("{:?}", msg);
            dbg!(msg.map(|(_, m)| (m.verify(), m.cmd_type(), m.subsystem(), m.cmd_id())));
        }
    }
}

#[derive(Debug)]
struct Message<'a> {
    command_id: [u8; 2],
    data: &'a [u8],
    check: u8,
}

impl<'a> Message<'a> {
    pub fn parse(src: &'a [u8]) -> IResult<&'a [u8], Self> {
        use nom::{bytes::streaming::*, number::streaming::*, sequence::*};

        let (src, len) = preceded(tag([0xfe]), u8)(src)?;
        let (src, (command_id, data, check)) = tuple((be_u16, take(len), u8))(src)?;

        Ok((src, Message {
            command_id: command_id.to_be_bytes(),
            data,
            check,
        }))
    }

    pub fn verify(&self) -> Result<(), ()> {
        let mut check = 0u8;
        check ^= self.command_id[0];
        check ^= self.command_id[1];
        check ^= self.data.len() as u8;
        check = self.data.iter().fold(check, |acc, b| acc ^ b);
        check ^= self.check;

        if check == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn cmd_type(&self) -> Result<CmdType, ()> {
        CmdType::try_from(self.command_id[0] >> 5)
    }

    pub fn subsystem(&self) -> Result<Subsystem, ()> {
        Subsystem::try_from(self.command_id[0] & 0x1f)
    }

    pub fn cmd_id(&self) -> u8 {
        self.command_id[1]
    }
}

#[derive(Debug)]
enum CmdType {
    Poll,
    SyncRequest,
    AsyncRequest,
    SyncResponse,
}

impl TryFrom<u8> for CmdType {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x00 => Ok(CmdType::Poll),
            0x01 => Ok(CmdType::SyncRequest),
            0x02 => Ok(CmdType::AsyncRequest),
            0x03 => Ok(CmdType::SyncResponse),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
enum Subsystem {
    Sys,
    Mac,
    Nwk,
    Af,
    Zdo,
    Sapi,
    Util,
    Debug,
    App,
    AppConfig,
    GreenPower,
}

impl TryFrom<u8> for Subsystem {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Subsystem::Sys),
            0x02 => Ok(Subsystem::Mac),
            0x03 => Ok(Subsystem::Nwk),
            0x04 => Ok(Subsystem::Af),
            0x05 => Ok(Subsystem::Zdo),
            0x06 => Ok(Subsystem::Sapi),
            0x07 => Ok(Subsystem::Util),
            0x08 => Ok(Subsystem::Debug),
            0x09 => Ok(Subsystem::App),
            0x0f => Ok(Subsystem::AppConfig),
            0x15 => Ok(Subsystem::GreenPower),
            _ => Err(()),
        }
    }
}
