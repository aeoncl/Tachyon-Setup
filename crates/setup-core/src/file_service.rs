use crate::error::TachyonInstallerError;
use std::path::{Path, PathBuf};
use shortcuts_rs::ShellLink;
use winapi::shared::winerror::S_OK;
use winapi::um::shlobj::{SHGetFolderPathW, CSIDL_COMMON_PROGRAMS, SHGFP_TYPE_CURRENT};

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
        Self::remove_start_menu_shortcut(&log)?;
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

    pub fn create_start_menu_shortcut(
        wl_install_folder: &Path,
        log: impl Fn(String),
    ) -> Result<(), TachyonInstallerError> {
        let target = wl_install_folder.join("Messenger").join("draal.exe");


        let shortcut_dir = Self::common_programs_dir()
            .ok_or(TachyonInstallerError::PathNotExist("CSIDL_COMMON_PROGRAMS".into()))?
            .join("Tachyon");
        
        std::fs::create_dir_all(&shortcut_dir)?;

        let shortcut_path = shortcut_dir.join("Windows Live Messenger (Tachyon).lnk");

        log(format!("Creating Start Menu entry: {}", shortcut_path.display()));

        // icon_location points to the exe itself so the shortcut inherits its icon
        let sl = ShellLink::new(
            target,
            None,
            Some("Windows Live Messenger (Tachyon)".to_string()),
            None
        ).map_err(|e| TachyonInstallerError::PathNotExist(format!("ShellLink: {:?}", e)))?;

        sl.create_lnk(&shortcut_path)
            .map_err(|e| TachyonInstallerError::PathNotExist(format!("create_lnk: {:?}", e)))?;

        Ok(())
    }
    
    pub fn remove_start_menu_shortcut(
        log: impl Fn(String),
    ) -> Result<(), TachyonInstallerError> {

        let shortcut_dir = Self::common_programs_dir()
            .ok_or(TachyonInstallerError::PathNotExist("CSIDL_COMMON_PROGRAMS".into()))?
            .join("Tachyon");

        if shortcut_dir.exists() {
            log(format!("Removing Start Menu entry: {}", shortcut_dir.display()));
            let _ = std::fs::remove_dir_all(&shortcut_dir);
        }
        Ok(())
    }

    fn common_programs_dir() -> Option<PathBuf> {
        let mut buf = [0u16; 260]; // MAX_PATH
        let hr = unsafe {
            SHGetFolderPathW(
                std::ptr::null_mut(),
                CSIDL_COMMON_PROGRAMS as i32,
                std::ptr::null_mut(),
                SHGFP_TYPE_CURRENT,
                buf.as_mut_ptr(),
            )
        };
        if hr != S_OK {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        use std::os::windows::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
    }

}
