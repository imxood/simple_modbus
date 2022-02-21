use anyhow::Result;
use std::{
    io::{Read, Write},
    time::Duration,
};

pub trait Stream: Read + Write {
    /// 设置 数据传输 的超时时间
    fn set_timeout(&mut self, timeout: Duration) -> Result<()>;
}
