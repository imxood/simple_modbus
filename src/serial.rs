use anyhow::Result;
use serialport::SerialPort;
use std::{
    io::{Read, Write},
    time::Duration,
};

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

    /// 设置 串口数据读写 的超时时间
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.inner.set_timeout(timeout)?;
        Ok(())
    }

    /// 检查串口是否有效
    pub fn available() -> Result<Vec<String>> {
        let ports = serialport::available_ports()?;
        let ports = ports
            .iter()
            .map(|port| port.port_name.clone())
            .collect::<Vec<String>>();
        Ok(ports)
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
    fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.inner.set_timeout(timeout)?;
        Ok(())
    }
}
