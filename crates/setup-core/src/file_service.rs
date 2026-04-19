use crate::error::TachyonInstallerError;
use std::path::Path;

pub const MSN_MSGR_FILE_NAMES: &[&str] = &[
    "zathras.dll",
    "draal.exe",
    "draal.ini",
    "epsilon3.dll",
    "tachyon.exe",
];

pub const CONTACT_FILE_NAMES: &[&str] = &[
    "zathras.dll",
    "draal.exe",
    "draal.ini"
];

pub struct FileService;

impl FileService {
    pub fn uninstall(path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        let messenger_path = path.join("Messenger");
        Self::remove_files(
            &messenger_path,
            MSN_MSGR_FILE_NAMES,
            "Untachyonyzing Windows Live Messenger.",
            &log,
        )?;

        let contacts_path = path.join("Contacts");
        Self::remove_files(
            &contacts_path,
            CONTACT_FILE_NAMES,
            "Nerfing Windows Live Contacts.",
            &log,
        )?;

        Self::remove_tachyon_idcrl_files(&log)?;
        Ok(())
    }

    /// Check whether the given folder looks like a valid WLM 2009 (14.0) install.
    pub fn is_valid_install_folder(path: &Path) -> Result<bool, TachyonInstallerError> {
        let contacts_path = path.join("Contacts");
        let messenger_path = path.join("Messenger");

        if !contacts_path.exists() || !messenger_path.exists() {
            return Ok(false);
        }

        let messenger_folder_content = messenger_path.read_dir()?;

        for messenger_file in messenger_folder_content.filter_map(|f| f.ok()) {
            let messenger_file = messenger_file.file_name();
            if let Some(name) = messenger_file.to_str() {
                if name.starts_with("msgrapp.14.0") {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check whether Tachyon has already been installed into this WLM folder.
    pub fn is_installed(path: &Path) -> bool {
        let messenger_path = path.join("Messenger");
        let read = match messenger_path.read_dir() {
            Ok(r) => r,
            Err(_) => return false,
        };

        for entry in read.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name == "tachyon.exe" {
                    return true;
                }
            }
        }
        false
    }

    fn remove_files(
        folder: &Path,
        names: &[&str],
        header: &str,
        log: impl Fn(String),
    ) -> Result<(), TachyonInstallerError> {
        log(header.into());
        for name in names {
            let file_path = folder.join(name);
            if file_path.exists() {
                std::fs::remove_file(file_path)?;
                log(format!("{}\\{}", folder.display(), name));
            } else {
                log(format!("File not found: {}, skipping...", name));
            }
        }
        Ok(())
    }

    fn remove_tachyon_idcrl_files(log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Removing Tachyon IdentityCRL files.".into());
        let project_dir = directories::ProjectDirs::from("", "Microsoft", "IdentityCRL");
        if let Some(project_dir) = project_dir {
            let tachyon_idcrl_dir = project_dir.data_dir().parent().map(|p| p.join("Tachyon"));
            if let Some(tachyon_idcrl_dir) = tachyon_idcrl_dir {
                if tachyon_idcrl_dir.exists() {
                    std::fs::remove_dir_all(tachyon_idcrl_dir)?;
                }
            }
        }
        Ok(())
    }
}
