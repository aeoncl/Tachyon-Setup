use thiserror::Error;

#[derive(Error, Debug)]
pub enum BinPatcherError {

    #[error("File IO Error: {}", .0)]
    Io(#[from] std::io::Error),
    #[error("Could not convert bytes to UTF8 String: {}", .0)]
    FromUTF8(#[from] std::string::FromUtf8Error),
    #[error("Patch had unexepected result for binary: {} (address: {}, expected {} but was {}", .binary_path, .address, .expected, .actual)]
    UnexpectedStringPatch {binary_path: String, address: u64, expected: String,  actual: String},
    #[error("Patch had unexepected result for binary: {} (address: {}, expected {:?} but was {:?}", .binary_path, .address, .expected, .actual)]
    UnexpectedBytePatch {binary_path: String, address: u64, expected: Vec<u8>,  actual: Vec<u8>},
    #[error("Patch aborted due to invalid original content for binary: {} (address: {}, expected {} but was {}", .binary_path, .address, .expected, .actual)]
    UnexpectedBinaryStringContent {binary_path: String, address: u64, expected: String,  actual: String},
    #[error("Patch aborted due to invalid original content for binary: {} (address: {}, expected {:?} but was {:?}", .binary_path, .address, .expected, .actual)]
    UnexpectedBinaryContent {binary_path: String, address: u64, expected: Vec<u8>,  actual: Vec<u8>}
}

