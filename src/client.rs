use std::{fmt, io, str::FromStr};

pub trait Client {
    fn read_discrete_inputs(&mut self, address: u16, quantity: u16) -> Result<Vec<Coil>>;

    fn read_coils(&mut self, address: u16, quantity: u16) -> Result<Vec<Coil>>;

    fn write_single_coil(&mut self, address: u16, value: Coil) -> Result<()>;

    fn write_multiple_coils(&mut self, address: u16, coils: &[Coil]) -> Result<()>;

    fn read_input_registers(&mut self, address: u16, quantity: u16) -> Result<Vec<u16>>;

    fn read_holding_registers(&mut self, address: u16, quantity: u16) -> Result<Vec<u16>>;

    fn write_single_register(&mut self, address: u16, value: u16) -> Result<()>;

    fn write_multiple_registers(&mut self, address: u16, values: &[u16]) -> Result<()>;

    fn set_uid(&mut self, uid: u8);
}

/// `InvalidData` reasons
#[derive(Debug)]
pub enum Reason {
    UnexpectedReplySize,
    BytecountNotEven,
    SendBufferEmpty,
    RecvBufferEmpty,
    SendBufferTooBig,
    DecodingError,
    EncodingError,
    InvalidByteorder,
    Custom(String),
}

/// Combination of Modbus, IO and data corruption errors
#[derive(Debug)]
pub enum Error {
    Exception(ExceptionCode),
    Io(io::Error),
    InvalidResponse,
    InvalidData(Reason),
    InvalidFunction,
    ParseCoilError,
    ParseInfoError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match *self {
            Exception(ref code) => write!(f, "modbus exception: {:?}", code),
            Io(ref err) => write!(f, "I/O error: {}", err),
            InvalidResponse => write!(f, "invalid response"),
            InvalidData(ref reason) => write!(f, "invalid data: {:?}", reason),
            InvalidFunction => write!(f, "invalid modbus function"),
            ParseCoilError => write!(f, "parse coil could not be parsed"),
            ParseInfoError => write!(f, "failed parsing device info as utf8"),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        use Error::*;

        match *self {
            Exception(_) => "modbus exception",
            Io(_) => "I/O error",
            InvalidResponse => "invalid response",
            InvalidData(_) => "invalid data",
            InvalidFunction => "invalid modbus function",
            ParseCoilError => "parse coil could not be parsed",
            ParseInfoError => "failed parsing device info as utf8",
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
/// Modbus exception codes returned from the server.
pub enum ExceptionCode {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    SlaveOrServerFailure = 0x04,
    Acknowledge = 0x05,
    SlaveOrServerBusy = 0x06,
    NegativeAcknowledge = 0x07,
    MemoryParity = 0x08,
    NotDefined = 0x09,
    GatewayPath = 0x0a,
    GatewayTarget = 0x0b,
}

impl From<ExceptionCode> for Error {
    fn from(err: ExceptionCode) -> Error {
        Error::Exception(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

/// Result type used to nofify success or failure in communication
pub type Result<T> = std::result::Result<T, Error>;

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

impl FromStr for Coil {
    type Err = Error;
    fn from_str(s: &str) -> Result<Coil> {
        if s == "On" {
            Ok(Coil::On)
        } else if s == "Off" {
            Ok(Coil::Off)
        } else {
            Err(Error::ParseCoilError)
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
