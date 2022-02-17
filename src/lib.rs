// pub mod client;
// pub mod config;
pub mod serial;
pub mod stream;

use std::process::id;

use anyhow::Result;
use bytes::{BufMut, Bytes, BytesMut};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
};
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

impl From<Function> for Bytes {
    fn from(req: Function) -> Bytes {
        let cnt = req.request_byte_count();
        let mut data = BytesMut::with_capacity(cnt);
        let code = req.code();
        match req {
            Function::ReadHoldingRegisters(id, address, quantity) => {
                data.put_u8(id);
                data.put_u8(code);

                data.put_u16(address);
                data.put_u16(quantity);
            }
            Function::WriteSingleRegister(id, address, word) => {
                data.put_u8(id);
                data.put_u8(code);

                data.put_u16(address);
                data.put_u16(word);
            }
            Function::WriteMultipleRegisters(id, address, words) => {
                data.put_u8(id);
                data.put_u8(code);

                data.put_u16(address);
                let len = words.len();
                data.put_u16(len as u16);
                data.put_u8((len * 2) as u8);
                for w in words {
                    data.put_u16(w);
                }
            }
        }
        let crc = calc_crc(&data);
        data.put_u16(crc);
        data.freeze()
    }
}

impl Function {
    pub(crate) fn request_byte_count(&self) -> usize {
        match *self {
            Self::ReadHoldingRegisters(_, _, _) | Self::WriteSingleRegister(_, _, _) => 8,
            Self::WriteMultipleRegisters(_, _, ref data) => 9 + data.len() * 2,
        }
    }

    pub(crate) fn code(&self) -> u8 {
        match *self {
            Self::ReadHoldingRegisters(_, _, _) => 0x03,
            Self::WriteSingleRegister(_, _, _) => 0x06,
            Self::WriteMultipleRegisters(_, _, _) => 0x10,
        }
    }
}

struct Client {
    stream: Box<dyn Stream>,
}

impl Client {
    fn new(stream: Box<dyn Stream>) -> Result<Self> {
        Ok(Self { stream })
    }

    // fn read(&self, fun: &Function) -> Result<Bytes> {}

    fn write(&mut self, fun: Function) -> Result<()> {
        let (w_buf, mut r_buf) = Self::build_buffer(&fun)?;
        match self.stream.write_all(&w_buf) {
            Ok(_) => match self.stream.read(&mut r_buf) {
                Ok(_) => {
                    log::info!("read buf: {:?}", &r_buf);
                }
                Err(e) => return Err(anyhow::anyhow!("传输异常, e: {:?}", &e)),
            },
            Err(e) => return Err(anyhow::anyhow!("传输异常, e: {:?}", &e)),
        }
        Ok(())
    }

    fn build_buffer(fun: &Function) -> Result<(Bytes, BytesMut)> {
        let (w_buf, r_buf) = match fun {
            Function::WriteSingleRegister(id, addr, data) => {
                // id + code + addr + data + crc
                // 4 + 2 + 2 ==> 6 + 2
                let mut w_buf = BytesMut::with_capacity(6 + 2);
                w_buf.put_u8(*id);
                w_buf.put_u8(0x05);
                w_buf.put_u16(*addr);
                w_buf.put_u16(*data);
                let crc = calc_crc(&w_buf);
                w_buf.put_u16(crc);

                let r_buf = BytesMut::with_capacity(8);
                (w_buf, r_buf)
            }
            Function::WriteMultipleRegisters(id, addr, data) => {
                let mut w_buf = BytesMut::with_capacity(6 + 2 * data.len());
                w_buf.put_u8(*id);
                w_buf.put_u8(0x05);
                w_buf.put_u16(*addr);
                for d in data {
                    w_buf.put_u16(*d);
                }
                let crc = calc_crc(&w_buf);
                w_buf.put_u16(crc);

                let r_buf = BytesMut::with_capacity(8);
                (w_buf, r_buf)
            }
            Function::ReadHoldingRegisters(id, addr, quantity) => {
                let mut w_buf = BytesMut::with_capacity(6 + 2);
                w_buf.put_u8(*id);
                w_buf.put_u8(0x03);
                w_buf.put_u16(*addr);
                w_buf.put_u16(*quantity);
                let crc = calc_crc(&w_buf);
                w_buf.put_u16(crc);

                let r_buf = BytesMut::with_capacity(6 + *quantity as usize * 2);
                (w_buf, r_buf)
            }
        };

        if w_buf.is_empty() {
            return Err(anyhow::anyhow!("无效的数据: 发送的数据为空"));
        }

        if w_buf.len() > MODBUS_MAX_PACKET_SIZE {
            return Err(anyhow::anyhow!("无效的数据: 发送的数据长度太大"));
        }

        Ok((w_buf.freeze(), r_buf))
    }

    // fn read_holding_registers(&mut self, id: u8, address: u16, quantity: u16) -> Result<Vec<u16>> {
    //     self.write(Function::ReadHoldingRegisters(id, address, quantity))
    // }

    fn write_single_register(&mut self, id: u8, address: u16, value: u16) -> Result<()> {
        self.write(Function::WriteSingleRegister(id, address, value))
    }

    fn write_multiple_registers(&mut self, id: u8, address: u16, values: Vec<u16>) -> Result<()> {
        self.write(Function::WriteMultipleRegisters(id, address, values))
    }
}

// pub enum Response {
//     /// 读指定数量的保持寄存器的数据
//     ReadHoldingRegisters(Id, Vec<Word>),

//     /// 写单个寄存器
//     WriteSingleRegister(Id, Address, Word),

//     /// 写多个寄存器
//     WriteMultipleRegisters(Id, Address, Quantity),
// }

// impl Response {
//     pub fn decode(&self) -> Bytes {
//         let cnt = self.response_byte_count();
//         let mut data = BytesMut::with_capacity(cnt);
//         bytes.put_u8(self.code());
//         match *self {
//             Self::ReadHoldingRegisters(registers) => {
//                 data.put_u8(u8_len(registers.len() * 2));
//                 for r in registers {
//                     data.put_u16(r);
//                 }
//             }
//             Self::WriteSingleRegister(address, word) => {
//                 data.put_u16(address);
//                 data.put_u16(word);
//             }
//             Self::WriteMultipleRegisters(address, quantity) => {
//                 data.put_u16(address);
//                 data.put_u16(quantity);
//             }
//         }
//         data.freeze()
//     }

//     fn response_byte_count(&self) -> usize {
//         match *self {
//             Self::ReadHoldingRegisters(_, ref data) => 2 + data.len() * 2,
//             Self::WriteSingleRegister(_, _, _) | Self::WriteMultipleRegisters(_, _) => 5,
//         }
//     }

//     fn code(&self) -> u8 {
//         match *self {
//             Self::ReadHoldingRegisters(_, _) => 0x03,
//             Self::WriteSingleRegister(_, _, _) => 0x06,
//             Self::WriteMultipleRegisters(_, _, _) => 0x10,
//         }
//     }
// }

fn calc_crc(data: &[u8]) -> u16 {
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

#[test]
fn test_request() {
    let Client = Client::new(SerialStream);
    let req = Function::ReadHoldingRegisters(15, 0x1122, 2);
    let data = Bytes::from(req);
}
