use std::time::Duration;

#[derive(Clone, Copy)]
pub struct Config {
    /// Connection timeout for TCP socket (Default: `OS Default`)
    pub connect_timeout: Option<Duration>,
    /// Timeout when reading from the TCP socket (Default: `infinite`)
    pub read_timeout: Option<Duration>,
    /// Timeout when writing to the TCP socket (Default: `infinite`)
    pub write_timeout: Option<Duration>,
    /// The modbus Unit Identifier used in the modbus layer (Default: `1`)
    pub modbus_uid: u8,
}
