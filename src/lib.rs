// pub mod client;
// pub mod config;
pub mod serial;
pub mod stream;

use std::{io::Cursor, process::id, str::FromStr, time::Duration};

use anyhow::Result;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
};
use serial::SerialStream;
use stream::Stream;

/// Modbus从设备 寄存器地址
pub(crate) type Address = u16;

/// Modbus从设备 Id
pub(crate) type Id = u8;

/// Modbus使用16位数表示它的数据项(大端模式)
pub(crate) type Word = u16;

/// Modbus需要读或写的数据数量
pub(crate) type Quantity = u16;

const MODBUS_MAX_PACKET_SIZE: usize = 260;

pub enum Function {
    /// 读指定数量的保持寄存器的数据
    /// (modbus从设备ID, 要读的保持寄存器的起始地址, 要读的保持寄存器的数量)
    ReadHoldingRegisters(Id, Address, Quantity),

    /// 写单个寄存器
    /// (modbus从设备ID, 要写入的寄存器地址, 要写入这个寄存器的单个数据)
    WriteSingleRegister(Id, Address, Word),

    /// 写多个寄存器
    /// (modbus从设备ID, 要写入的寄存器的起始地址, 要写入这些寄存器的数据列表)
    WriteMultipleRegisters(Id, Address, Vec<Word>),
}

pub struct Client {
    stream: Box<dyn Stream>,
    need_reply: bool,
}

impl Client {
    pub fn new(stream: Box<dyn Stream>) -> Result<Self> {
        Ok(Self {
            stream,
            need_reply: true,
        })
    }

    pub fn read_holding_registers(
        &mut self,
        id: Id,
        address: Address,
        quantity: Quantity,
    ) -> Result<Vec<Word>> {
        let bytes = self.read(Function::ReadHoldingRegisters(id, address, quantity))?;
        pack_bytes(bytes)
    }

    pub fn write_single_register(&mut self, id: Id, address: Address, value: Word) -> Result<()> {
        self.write(Function::WriteSingleRegister(id, address, value))
    }

    pub fn write_multiple_registers(
        &mut self,
        id: Id,
        address: Address,
        values: Vec<Word>,
    ) -> Result<()> {
        self.write(Function::WriteMultipleRegisters(id, address, values))
    }

    pub fn set_need_reply(&mut self, need_reply: bool) {
        self.need_reply = need_reply;
    }

    fn get_reply_data(&self, mut reply: Bytes) -> Result<Bytes> {
        if reply.len() <= 5 {
            return Err(anyhow::anyhow!("数据异常, 没有取到有效的数据"));
        }
        let len = *reply.get(2).unwrap();
        if 5 + len as usize != reply.len() {
            return Err(anyhow::anyhow!("数据异常, 没有取到有效的数据"));
        }

        let _ = reply.split_to(3);
        Ok(reply.split_to(len as usize))
    }

    fn validate_reply(&self, req: &Bytes, reply: &BytesMut) -> Result<()> {
        let req_len = req.len();
        let reply_len = reply.len();

        // 检查数据长度, 仅仅简单的判断一下
        if req_len < 3 || reply_len < 3 {
            return Err(anyhow::anyhow!("数据异常"));
        }

        // 检查ID
        if req.get(0) != reply.get(0) {
            return Err(anyhow::anyhow!("数据异常, 响应ID与请求ID不一致"));
        }

        // 检查功能码
        if req.get(1) != reply.get(1) {
            return Err(anyhow::anyhow!("数据异常, 响应功能码与请求功能码不一致"));
        }

        // 检查reply的CRC
        let crc = ((reply[reply_len - 2] as u16) << 8) + (reply[reply_len - 1] as u16);
        let (data, _) = reply.split_at(reply_len - 2);
        if crc != calc_crc(data) {
            return Err(anyhow::anyhow!("数据异常, 响应数据CRC错误"));
        }
        Ok(())
    }

    fn transfer(&mut self, req: &Bytes, reply: &mut BytesMut) -> Result<()> {
        match self.stream.write_all(req) {
            Ok(_) => {
                if !self.need_reply {
                    self.stream.read(reply).ok();
                    return Ok(());
                }

                match self.stream.read(reply) {
                    Ok(_) => {
                        self.validate_reply(req, reply)?;
                    }
                    Err(e) => return Err(anyhow::anyhow!("传输异常, e: {:?}", &e)),
                }
            }
            Err(e) => return Err(anyhow::anyhow!("传输异常, e: {:?}", &e)),
        }
        Ok(())
    }

    fn read(&mut self, fun: Function) -> Result<Bytes> {
        let (req, mut reply) = Self::build_buffer(fun)?;
        self.transfer(&req, &mut reply)?;
        self.get_reply_data(reply.freeze())
    }

    fn write(&mut self, fun: Function) -> Result<()> {
        let (req, mut reply) = Self::build_buffer(fun)?;
        self.transfer(&req, &mut reply)
    }

    fn build_buffer(fun: Function) -> Result<(Bytes, BytesMut)> {
        let (req, reply) = match fun {
            Function::WriteSingleRegister(id, addr, data) => {
                let mut req = BytesMut::with_capacity(6 + 2);
                req.put_u8(id);
                req.put_u8(0x06);
                req.put_u16(addr);
                req.put_u16(data);
                let crc = calc_crc(&req);
                req.put_u16(crc);

                let reply = vec![0u8; 8];
                let reply = BytesMut::from(&reply[..]);
                (req, reply)
            }
            Function::WriteMultipleRegisters(id, addr, data) => {
                let word_cnt = data.len() as u16;
                let byte_cnt = 2 * word_cnt as u8;
                let mut req = BytesMut::with_capacity(7 + byte_cnt as usize + 2);
                req.put_u8(id);
                req.put_u8(0x10);
                req.put_u16(addr);
                req.put_u16(word_cnt);
                req.put_u8(byte_cnt);
                for d in data {
                    req.put_u16(d);
                }
                let crc = calc_crc(&req);
                req.put_u16(crc);

                let reply = vec![0u8; 8];
                let reply = BytesMut::from(&reply[..]);
                (req, reply)
            }
            Function::ReadHoldingRegisters(id, addr, quantity) => {
                let mut req = BytesMut::with_capacity(6 + 2);
                req.put_u8(id);
                req.put_u8(0x03);
                req.put_u16(addr);
                req.put_u16(quantity);
                let crc = calc_crc(&req);
                req.put_u16(crc);

                let reply = vec![0u8; 5 + quantity as usize * 2];
                let reply = BytesMut::from(&reply[..]);
                (req, reply)
            }
        };

        if req.is_empty() {
            return Err(anyhow::anyhow!("无效的数据: 发送的数据为空"));
        }

        if req.len() > MODBUS_MAX_PACKET_SIZE {
            return Err(anyhow::anyhow!("无效的数据: 发送的数据长度太大"));
        }

        Ok((req.freeze(), reply))
    }
}

pub fn calc_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in data {
        crc ^= u16::from(*x);
        for _ in 0..8 {
            let crc_odd = (crc & 0x0001) != 0;
            crc >>= 1;
            if crc_odd {
                crc ^= 0xA001;
            }
        }
    }
    crc << 8 | crc >> 8
}

pub fn pack_bytes(mut bytes: Bytes) -> Result<Vec<u16>> {
    let size = bytes.len();
    if size % 2 != 0 {
        return Err(anyhow::anyhow!("无效的数据, 字节数据非偶数"));
    }

    let mut res = Vec::with_capacity(size / 2 + 1);
    for _ in 0..size / 2 {
        res.push(bytes.get_u16());
    }
    Ok(res)
}

pub fn unpack_bytes(data: &[u16]) -> Vec<u8> {
    let size = data.len();
    let mut res = Vec::with_capacity(size * 2);
    for b in data {
        res.push((*b >> 8 & 0xff) as u8);
        res.push((*b & 0xff) as u8);
    }
    res
}

pub fn pack_bits(bits: &[Coil]) -> Vec<u8> {
    let bitcount = bits.len();
    let packed_size = bitcount / 8 + if bitcount % 8 > 0 { 1 } else { 0 };
    let mut res = vec![0; packed_size];
    for (i, b) in bits.iter().enumerate() {
        let v = match *b {
            Coil::On => 1u8,
            Coil::Off => 0u8,
        };
        res[(i / 8) as usize] |= v << (i % 8);
    }
    res
}

pub fn unpack_bits(bytes: &[u8], count: u16) -> Vec<Coil> {
    let mut res = Vec::with_capacity(count as usize);
    for i in 0..count {
        if (bytes[(i / 8u16) as usize] >> (i % 8)) & 0b1 > 0 {
            res.push(Coil::On);
        } else {
            res.push(Coil::Off);
        }
    }
    res
}

/// Single bit status values, used in read or write coil functions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coil {
    On,
    Off,
}

impl Coil {
    fn code(self) -> u16 {
        match self {
            Coil::On => 0xff00,
            Coil::Off => 0x0000,
        }
    }
}

impl From<bool> for Coil {
    fn from(b: bool) -> Coil {
        if b {
            Coil::On
        } else {
            Coil::Off
        }
    }
}

impl std::ops::Not for Coil {
    type Output = Coil;

    fn not(self) -> Coil {
        match self {
            Coil::On => Coil::Off,
            Coil::Off => Coil::On,
        }
    }
}

#[test]
fn test_function() -> Result<()> {
    std::env::set_var("RUST_LOG", "DEBUG");
    env_logger::init();

    let stream = Box::new(SerialStream::new("COM4", 19200)?);

    let mut client = Client::new(stream)?;
    client.set_need_reply(false);

    let id = 15;

    loop {
        if let Err(e) = client.write_single_register(id, 0x0000, 0x01) {
            log::error!("{:?}", &e);
        }

        let pos = 45678u32 * 3;
        log::info!("pos: {:?}", &pos);

        if let Err(e) = client.write_multiple_registers(
            id,
            0x0016,
            vec![(pos & 0xffff) as u16, (pos >> 16) as u16],
        ) {
            log::error!("{:?}", &e);
        }
        std::thread::sleep(Duration::from_secs(3));

        match client.read_holding_registers(id, 0x0016, 2) {
            Err(e) => {
                log::error!("{:?}", &e);
            }
            Ok(data) => {
                log::info!("data: {:?}", &data);
            }
        }

        let pos = 0;
        if let Err(e) = client.write_multiple_registers(
            id,
            0x0016,
            vec![(pos & 0xffff) as u16, (pos >> 16) as u16],
        ) {
            log::error!("{:?}", &e);
        }
        std::thread::sleep(Duration::from_secs(3));

        match client.read_holding_registers(id, 0x0016, 2) {
            Err(e) => {
                log::error!("{:?}", &e);
            }
            Ok(data) => {
                log::info!("data: {:?}", &data);
            }
        }
    }
}
