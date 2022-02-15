use std::io::{Write, Read};

use anyhow::Result;

pub trait Stream: Read + Write {
    fn try_read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn readable(&self) -> Result<()>;

    fn try_write(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn writable(&self) -> Result<()>;
}
