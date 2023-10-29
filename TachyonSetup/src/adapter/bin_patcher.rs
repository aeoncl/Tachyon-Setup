use std::{path::PathBuf, fs::{File, OpenOptions, self}, io::{Read, SeekFrom, Seek, Write}};

use super::bin_patcher_error::BinPatcherError;

pub fn check_create_backup(file_path: &PathBuf) -> Result<(), BinPatcherError>  {
    let og_filename: String = format!("{}.og", file_path.file_name().expect("a final component to be present in path").to_string_lossy());
    let og_file_path = file_path.parent().expect("a final component to be present in path").join(og_filename);
    if !og_file_path.exists() {
        fs::copy(file_path, &og_file_path)?;
    }
    Ok(())
}

pub fn patch_bytes(file_path: &PathBuf, address: u64, bytes: &[u8]) -> Result<(), BinPatcherError> {
    check_create_backup(file_path)?;
    let mut file = OpenOptions::new().read(true).write(true).open(file_path)?;

    file.seek(SeekFrom::Start(address))?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_data()?;
    file.sync_all()?;

    Ok(())
}

pub fn patch_string(file_path: &PathBuf, address: u64, string: String) -> Result<(), BinPatcherError>  {
    patch_bytes(file_path, address, string.as_bytes())
}

pub fn read_bytes(file_path: &PathBuf, address: u64, len: usize) -> Result<Vec<u8>, BinPatcherError> {
    let mut file = OpenOptions::new().read(true).write(false).open(file_path)?;

    let mut buffer: Vec<u8> = vec![0; len];

    file.seek(SeekFrom::Start(address))?;
    file.read_exact(buffer.as_mut_slice())?;

    Ok(buffer)
} 

pub fn read_string(file_path: &PathBuf, address: u64, len: usize) -> Result<String, BinPatcherError>  {
    let bytes = read_bytes(file_path, address, len)?;
    Ok(String::from_utf8(bytes)?)
}






