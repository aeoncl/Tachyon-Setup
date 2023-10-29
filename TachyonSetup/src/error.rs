use thiserror::Error;

use crate::adapter::bin_patcher_error;


#[derive(Error, Debug)]
pub enum TachyonInstallerError {
    #[error("An error has occured while accessing a registry key: {}", .0)]
    RegistryKey(#[from] registry::key::Error),
    #[error("An error has occured while accessing a registry value: {}", .0)]
    RegistryValue(#[from] registry::value::Error),
    #[error("Path doesn't exist: {}", .0)]
    PathNotExist(String),
    #[error(transparent)]
    BinPatcher(#[from] bin_patcher_error::BinPatcherError)
}
