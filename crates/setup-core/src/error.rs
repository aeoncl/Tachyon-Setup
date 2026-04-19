use thiserror::Error;

#[derive(Error, Debug)]
pub enum TachyonInstallerError {
    #[error("An error has occured while accessing a registry key: {}", .0)]
    RegistryKey(#[from] registry::key::Error),
    #[error("An error has occured while accessing a registry value: {}", .0)]
    RegistryValue(#[from] registry::value::Error),
    #[error("Path doesn't exist: {}", .0)]
    PathNotExist(String),
    #[error("Could not create file: {}", .0)]
    CouldNotCreateFile(#[from] std::io::Error),
    #[error("Could not enumerate running processes")]
    EnumerateProcess,
    #[error("Invalid path: {:?}", .0)]
    InvalidPath(Option<String>),
}
