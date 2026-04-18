use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use lazy_static::lazy_static;
use lazy_static_include::lazy_static_include_bytes;
use registry::{Data, Hive, Security};
use utfx::U16CString;
use crate::error::TachyonInstallerError;

lazy_static_include_bytes! {
    DRAAL => "./assets/draal.exe",
    DRAAL_INI_MSNMSGR => "./assets/draal_msnmsgr.ini",
    DRAAL_INI_CONTACTS => "./assets/draal_contacts.ini",
    EPSILON3 => "./assets/epsilon3.dll",
    ZATHRAS => "./assets/zathras.dll",
    TACHYON => "./assets/tachyon.exe",
    ESENT => "./assets/esent.dll",
    ESENTPRF => "./assets/esentprf.dll"
}

lazy_static! {
    static ref MSN_MSGR_FILES: Vec<FileEntry> = vec![
        FileEntry::new("zathras.dll", &ZATHRAS),
        FileEntry::new("draal.exe", &DRAAL),
        FileEntry::new("draal.ini", &DRAAL_INI_MSNMSGR),
        FileEntry::new("epsilon3.dll", &EPSILON3),
        FileEntry::new("tachyon.exe", &TACHYON),
    ];

    static ref CONTACT_FILES: Vec<FileEntry> = vec![
        FileEntry::new("zathras.dll", &ZATHRAS),
        FileEntry::new("draal.exe", &DRAAL),
        FileEntry::new("draal.ini", &DRAAL_INI_CONTACTS),
        FileEntry::new("esent.dll", &ESENT),
        FileEntry::new("esentprf.dll", &ESENTPRF),
    ];
}

struct FileEntry {
    name: String,
    data: &'static [u8],
}

impl FileEntry {

    pub fn new(name: &str, data: &'static [u8]) -> Self {
        Self {
            name: name.to_owned(),
            data,
        }
    }
    pub fn write_to_disk(&self, path: &Path) -> Result<(), TachyonInstallerError> {
        let path = path.join(&self.name);
        if let Some(path) = path.to_str() {
            let mut file = File::create(path)?;
            file.write_all(self.data)?;
            Ok(())
        } else {
            Err(TachyonInstallerError::InvalidPath(path.to_str().map(|s| s.to_owned())))
        }
    }
}

pub struct FileService {}




impl FileService {

    pub fn install(path: &PathBuf, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        let messenger_path = path.join("Messenger");
        Self::write_msnmsgr_files(&messenger_path, &log)?;
        let contacts_path = path.join("Contacts");
        Self::write_contact_files(&contacts_path, &log)?;

        Ok(())
    }

    pub fn uninstall(path: &PathBuf, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        let messenger_path = path.join("Messenger");
        Self::remove_msnmsgr_files(&messenger_path, &log)?;
        let contacts_path = path.join("Contacts");
        Self::remove_contact_files(&contacts_path, &log)?;

        Ok(())
    }

    pub fn is_valid_install_folder(path: &PathBuf) -> Result<bool, TachyonInstallerError> {
        let contacts_path = path.join("Contacts");
        let messenger_path = path.join("Messenger");

        if !contacts_path.exists() || !messenger_path.exists() {
            return Ok(false);
        }

        let messenger_folder_content = messenger_path.read_dir()?;

        for messenger_file in messenger_folder_content.filter_map(|messenger_file| messenger_file.ok()) {
            let messenger_file = messenger_file.file_name();
            if let Some(messenger_file) = messenger_file.to_str() {
                if messenger_file.starts_with("msgrapp.14.0") {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn is_installed(path: &PathBuf) -> bool {
        let messenger_path = path.join("Messenger");

        for messenger_file in messenger_path.read_dir().unwrap() {
            let messenger_file = messenger_file.unwrap();
            if messenger_file.file_name().to_str().unwrap() == "tachyon.exe" {
                return true;
            }
        }

        false
    }

    fn write_file(parent_path: &Path, file_entry: &FileEntry, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log(format!("Write: {}", file_entry.name).into());
        file_entry.write_to_disk(parent_path)
    }

    fn write_contact_files(path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Supercharging Windows Live Contacts files.".into());

        for file_entry in CONTACT_FILES.iter() {
            Self::write_file(path, file_entry, &log)?;
        }

        Ok(())
    }


    fn write_msnmsgr_files(path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Enriching Windows Live Messenger files.".into());

        for file_entry in MSN_MSGR_FILES.iter() {
            Self::write_file(path, file_entry, &log)?;
        }

        Ok(())
    }


    fn remove_contact_files(path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Nerfing Windows Live Contacts.".into());

        for file_entry in CONTACT_FILES.iter() {
            let file_path = path.join(&file_entry.name);
            if file_path.exists() {
                std::fs::remove_file(file_path)?;
            } else {
                log(format!("File not found: {}, skipping...", file_entry.name).into());
            }
        }

        Ok(())

    }

    fn remove_msnmsgr_files(path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Untachyonyzing Windows Live Messenger.".into());

        for file_entry in MSN_MSGR_FILES.iter() {
            let file_path = path.join(&file_entry.name);
            if file_path.exists() {
                std::fs::remove_file(file_path)?;
            } else {
                log(format!("File not found: {}, skipping...", file_entry.name).into());
            }
        }

        Ok(())
    }



}