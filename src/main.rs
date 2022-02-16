use bytes::Bytes;
use simple_modbus::Request;

fn main() {
    let data = Bytes::from(Request::ReadHoldingRegisters(1, 0x1122, 2));
    println!("data: {:?}", &data);
}
