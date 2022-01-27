use std::time::Duration;

use anyhow::Result;
use serialport::SerialPort;

struct SerialDevice {
    inner: Box<dyn SerialPort>,
}

impl SerialDevice {
    pub fn new(port: &str, baud_rate: u32) -> Result<Self> {
        if !Self::available(port)? {
            return Err(anyhow::anyhow!("找不到串口设备: {port}"));
        }

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
            return Ok(false);
        }
        Ok(true)
    }
}
