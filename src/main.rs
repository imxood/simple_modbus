use bytes::Bytes;
use simple_modbus::Function;

fn main() {
    let data = Bytes::from(Function::ReadHoldingRegisters(1, 0x1122, 2));
    println!("data: {:?}", &data);
}
