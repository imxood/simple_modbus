// pub mod client;
// pub mod config;
// pub mod serial;
// pub mod stream;

use anyhow::Result;
use bytes::{BufMut, Bytes, BytesMut};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
};

/// Modbus从设备 寄存器地址
pub(crate) type Address = u16;

/// Modbus从设备 Id
pub(crate) type Id = u8;

/// Modbus使用16位数表示它的数据项(大端模式)
pub(crate) type Word = u16;

/// Modbus需要读或写的数据数量
pub(crate) type Quantity = u16;

pub enum Request {
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

impl From<Request> for Bytes {
    fn from(req: Request) -> Bytes {
        let cnt = req.request_byte_count();
        let mut data = BytesMut::with_capacity(cnt);
        let code = req.code();
        match req {
            Request::ReadHoldingRegisters(id, address, quantity) => {
                data.put_u8(id);
                data.put_u8(code);

                data.put_u16(address);
                data.put_u16(quantity);
            }
            Request::WriteSingleRegister(id, address, word) => {
                data.put_u8(id);
                data.put_u8(code);

                data.put_u16(address);
                data.put_u16(word);
            }
            Request::WriteMultipleRegisters(id, address, words) => {
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

impl Request {
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
