use crate::error::TachyonInstallerError;
use std::path::{Path, PathBuf};
use winapi::DEFINE_GUID;
use winapi::shared::minwindef::LPVOID;
use winapi::shared::winerror::{S_FALSE, S_OK};
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC};
use winapi::um::objbase::COINIT_APARTMENTTHREADED;
use winapi::um::shlobj::{SHGetFolderPathW, CSIDL_COMMON_PROGRAMS, SHGFP_TYPE_CURRENT};
use winapi::um::shobjidl_core::IShellLinkW;
use winapi::um::objidl::IPersistFile;
use winapi::um::propkey::PKEY_AppUserModel_ID;
use winapi::um::propkeydef::REFPROPERTYKEY;
use winapi::um::propidl::{PROPVARIANT, REFPROPVARIANT};
use winapi::um::propsys::IPropertyStore;
use winapi::um::unknwnbase::IUnknown;
use winapi::shared::wtypes::{VT_LPWSTR, VARTYPE};
use winapi::ctypes::c_void;

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

/// Application User Model ID used for the Tachyon shortcut's taskbar/jump list.
const APP_USER_MODEL_ID: &str = "chat.tachyon.wave14";

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

        create_lnk(
            "Windows Live Messenger (Tachyon)",
            &target,
            Some(APP_USER_MODEL_ID),
            &shortcut_path,
        )?;

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

DEFINE_GUID!(CLSID_SHELLLINK, 0x00021401, 0x0000, 0x0000, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46);
DEFINE_GUID!(IID_ISHELLLINKW, 0x000214F9, 0x0000, 0x0000, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46);
DEFINE_GUID!(IID_IPERSISTFILE, 0x0000010B, 0x0000, 0x0000, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46);
DEFINE_GUID!(IID_IPROPERTIES, 0x886D8EEB, 0x8CF2, 0x4446, 0x8D, 0x02, 0xCD, 0xBA, 0x1D, 0xBD, 0xCF, 0x99);

fn to_wide(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide
}

fn create_lnk(
    name: &str,
    executable_path: &Path,
    app_model_id_override: Option<&str>,
    output_path: &Path,
) -> Result<(), TachyonInstallerError> {

    let desc_wide = to_wide(name);
    let exec_wide = to_wide(&executable_path.to_string_lossy());
    let out_wide = to_wide(&output_path.to_string_lossy());

    unsafe {
        // S_FALSE means COM is already initialized on this thread with a
        // compatible apartment model, which is fine to build on.
        let hr = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
        if hr != S_OK && hr != S_FALSE {
            return Err(TachyonInstallerError::PathNotExist(format!("CoInitializeEx: {:?}", hr)));
        }

        let mut shell_link_instance: *mut IShellLinkW = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_SHELLLINK,
            std::ptr::null_mut(),
            CLSCTX_INPROC,
            &IID_ISHELLLINKW,
            std::ptr::addr_of_mut!(shell_link_instance) as *mut LPVOID,
        );
        if hr != S_OK {
            return Err(TachyonInstallerError::PathNotExist("CoCreateInstance ShellLink".into()));
        }

        let app_model_id = app_model_id_override.map(|id| {
            let mut id_wide = to_wide(id);
            let mut propvar: PROPVARIANT = std::mem::zeroed();
            propvar.vt = VT_LPWSTR as VARTYPE;
            *propvar.data.pwszVal_mut() = id_wide.as_mut_ptr();
            propvar
        });

        // AppUserModelID as VT_LPWSTR, the property's canonical type. The store
        // deep-copies on SetValue, so the buffer only needs to outlive that call.

        let hr = shell_link_instance.as_mut_unchecked().SetPath(exec_wide.as_ptr());
        if hr != S_OK {
            return Err(TachyonInstallerError::PathNotExist(format!("SetPath: {:?}", hr)));
        }
        let _ = shell_link_instance.as_mut_unchecked().SetDescription(desc_wide.as_ptr());

        let iunknown = shell_link_instance as *mut IUnknown;
        let mut property_store: *mut IPropertyStore = std::ptr::null_mut();
        let ppv: *mut *mut c_void = std::ptr::addr_of_mut!(property_store) as *mut *mut c_void;
        let hr = iunknown.as_mut_unchecked().QueryInterface(&IID_IPROPERTIES, ppv);

        if hr != S_OK {
            return Err(TachyonInstallerError::PathNotExist("QueryInterface IPropertyStore".into()));
        }

        if let Some(app_model_id_prop) = app_model_id {
            let key: REFPROPERTYKEY = &PKEY_AppUserModel_ID;
            let propvar_ref: REFPROPVARIANT = &app_model_id_prop;
            let hr = property_store.as_mut_unchecked().SetValue(key, propvar_ref);
            if hr != S_OK {
                return Err(TachyonInstallerError::PathNotExist("IPropertyStore SetValue".into()));
            }
        }

        let hr = property_store.as_mut_unchecked().Commit();
        if hr != S_OK {
            return Err(TachyonInstallerError::PathNotExist(format!("IPropertyStore Commit: {:?}", hr)));
        }

        let mut persist: *mut IPersistFile = std::ptr::null_mut();
        let ppv: *mut *mut c_void = std::ptr::addr_of_mut!(persist) as *mut *mut c_void;
        let hr = iunknown.as_mut_unchecked().QueryInterface(&IID_IPERSISTFILE, ppv);

        if hr != S_OK {
            return Err(TachyonInstallerError::PathNotExist("QueryInterface IPersistFile".into()));
        }

        let hr = persist.as_mut_unchecked().Save(out_wide.as_ptr(), 1);
        if hr != S_OK {
            return Err(TachyonInstallerError::PathNotExist(format!("IPersistFile Save: {:?}", hr)));
        }

        let _ = property_store.as_mut_unchecked().Release();
        let _ = persist.as_mut_unchecked().Release();
        let _ = iunknown.as_mut_unchecked().Release();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a real shortcut through the COM path and asserts the
    /// AppUserModelID is embedded in the .lnk's property store data.
    #[test]
    fn shortcut_contains_app_model_id() {
        let shortcut_path = std::env::temp_dir().join("tachyon-create-lnk-test.lnk");
        let _ = std::fs::remove_file(&shortcut_path);

        create_lnk(
            "Windows Live Messenger (Tachyon)",
            Path::new("C:\\Windows\\System32\\cmd.exe"),
            None,
            &shortcut_path,
        )
        .expect("create_lnk failed");

        let bytes = std::fs::read(&shortcut_path).expect("shortcut was not written");

        // The .lnk property store serializes each property pair as
        // [size: u32][pid: u32][version: u8][vt: u16][padding: u16]
        // [char count: u32][UTF-16LE string], inside a storage whose header is
        // [storage size: u32]["1SPS" marker: u32][fmtid: GUID].
        let value: Vec<u8> = APP_USER_MODEL_ID
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        let value_offset = bytes
            .windows(value.len())
            .position(|window| window == value)
            .expect("AppUserModelID not found in the generated .lnk");

        assert_eq!(
            &bytes[value_offset - 4..value_offset],
            &((APP_USER_MODEL_ID.len() as u32) + 1).to_le_bytes(),
            "unexpected VT_LPWSTR char count"
        );
        assert_eq!(&bytes[value_offset - 8..value_offset - 6], &[0x1F, 0x00], "value is not VT_LPWSTR");
        assert_eq!(
            &bytes[value_offset - 13..value_offset - 9],
            &5u32.to_le_bytes(),
            "property id is not 5 (PKEY_AppUserModel_ID)"
        );
        let expected_fmtid: [u8; 16] = [
            0x55, 0x28, 0x4C, 0x9F, // Data1, little-endian
            0x79, 0x9F, // Data2
            0x39, 0x4B, // Data3
            0xA8, 0xD0, 0xE1, 0xD4, 0x2D, 0xE1, 0xD5, 0xF3,
        ];
        assert_eq!(
            &bytes[value_offset - 33..value_offset - 17],
            &expected_fmtid,
            "property set is not the AppUserModel property set"
        );

        std::fs::remove_file(&shortcut_path).unwrap();
    }
}
