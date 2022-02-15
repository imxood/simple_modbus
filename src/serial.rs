use std::time::Duration;

use anyhow::Result;
use serialport::SerialPort;

use crate::stream::Stream;

struct SerialStream {
    inner: Box<dyn SerialPort>,
}

impl SerialStream {
    pub fn new(port: &str, baud_rate: u32) -> Result<Self> {
        Self::available(port)?;

        let inner_device = serialport::new(port, baud_rate)
            .timeout(Duration::from_millis(10))
            .open()?;

        Ok(Self {
            inner: inner_device,
        })
    }

    pub fn available(addr: &str) -> Result<bool> {
        // 检查串口是否存在
        let ports = serialport::available_ports()?;
        if !ports.iter().any(|port| port.port_name == addr) {
            return Err(anyhow::anyhow!("找不到串口设备: {addr}"));
        }
        Ok(true)
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
