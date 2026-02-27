use std::fmt;

#[derive(Debug)]
pub enum MemoryError {
    NullPointer,
    InvalidLength,
    VirtualProtectFailed,
    ReadFailed,
    WriteFailed,
    OutOfBounds,
}

impl std::error::Error for MemoryError {}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::NullPointer => write!(f, "null pointer"),
            MemoryError::InvalidLength => write!(f, "invalid length"),
            MemoryError::VirtualProtectFailed => write!(f, "VirtualProtect failed"),
            MemoryError::ReadFailed => write!(f, "read failed"),
            MemoryError::WriteFailed => write!(f, "write failed"),
            MemoryError::OutOfBounds => write!(f, "out of bounds"),
        }
    }
}
