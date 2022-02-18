use std::{
    io::{Read, Write},
    time::Duration,
};

use anyhow::Result;
use serialport::SerialPort;

use crate::stream::Stream;

pub struct SerialStream {
    inner: Box<dyn SerialPort>,
}

impl SerialStream {
    pub fn new(port: &str, baud_rate: u32) -> Result<Self> {
        // Self::available(port)?;

        let inner_device = serialport::new(port, baud_rate)
            .timeout(Duration::from_millis(5000))
            .open()?;

        Ok(Self {
            inner: inner_device,
        })
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.inner.set_timeout(timeout)?;
        Ok(())
    }

    pub fn available(addr: &str) -> Result<()> {
        // 检查串口是否存在
        let ports = serialport::available_ports()?;
        if !ports.iter().any(|port| port.port_name == addr) {
            return Err(anyhow::anyhow!("找不到串口设备: {addr}"));
        }
        Ok(())
    }
}

impl Read for SerialStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for SerialStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Stream for SerialStream {
    fn try_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        todo!()
    }

    fn readable(&self) -> Result<()> {
        todo!()
    }

    fn try_write(&mut self, buf: &mut [u8]) -> Result<usize> {
        todo!()
    }

    fn writable(&self) -> Result<()> {
        todo!()
    }
}
