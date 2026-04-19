use lazy_static_include::lazy_static_include_bytes;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use setup_core::{TachyonInstallerError, CONTACT_FILE_NAMES, MSN_MSGR_FILE_NAMES};
use std::env;

lazy_static_include_bytes! {
    DRAAL => "./assets/draal.exe",
    DRAAL_INI_MSNMSGR => "./assets/draal_msnmsgr.ini",
    DRAAL_INI_CONTACTS => "./assets/draal_contacts.ini",
    EPSILON3 => "./assets/epsilon3.dll",
    ZATHRAS => "./assets/zathras.dll",
    TACHYON => "./assets/tachyon.exe",
    ESENT => "./assets/esent.dll",
    ESENTPRF => "./assets/esentprf.dll",
}

pub const UNINSTALLER_EXE: &[u8] = include_bytes!(env!("UNINSTALLER_EXE_PATH"));
pub const UNINSTALLER_EXE_NAME: &str = "tachyon_uninstaller.exe";

/// Maps a filename declared in tachyon_core::MSN_MSGR_FILE_NAMES to its bytes.
fn msn_msgr_bytes(name: &str) -> &'static [u8] {
    match name {
        "zathras.dll"  => &ZATHRAS,
        "draal.exe"    => &DRAAL,
        "draal.ini"    => &DRAAL_INI_MSNMSGR,
        "epsilon3.dll" => &EPSILON3,
        "tachyon.exe"  => &TACHYON,
        _ => {
            panic!("No embedded bytes for messenger file: {}", name)
        },
    }
}

/// Maps a filename declared in tachyon_core::CONTACT_FILE_NAMES to its bytes.
fn contact_bytes(name: &str) -> &'static [u8] {
    match name {
        "zathras.dll"  => &ZATHRAS,
        "draal.exe"    => &DRAAL,
        "draal.ini"    => &DRAAL_INI_CONTACTS,
        _ => {
            panic!("No embedded bytes for contact file: {}", name)
        },
    }
}

pub struct InstallerFileService;

impl InstallerFileService {
    pub fn install(
        path: &Path,
        log: impl Fn(String),
        progress: impl Fn(),
    ) -> Result<(), TachyonInstallerError> {
        let messenger_path = path.join("Messenger");
        log("Tachyonizing Windows Live Messenger.".into());
        for name in MSN_MSGR_FILE_NAMES {
            let bytes = msn_msgr_bytes(name);
            write_bytes(&messenger_path, name, bytes, &log)?;
            progress();
        }

        let contacts_path = path.join("Contacts");
        log("Supercharging Windows Live Contacts.".into());
        for name in CONTACT_FILE_NAMES {
            let bytes = contact_bytes(name);
            write_bytes(&contacts_path, name, bytes, &log)?;
            progress();
        }

        log("Unslopifying Microsoft Extensible Storage libraries.".into());
        if !contacts_path.join("esent.dll").exists() {
            write_bytes(&contacts_path, "esent.dll", *ESENT, &log)?;
        } else {
            log("esent.dll already exists, skipping.".into());
        }
        if !contacts_path.join("esentprf.dll").exists() {
            write_bytes(&contacts_path, "esentprf.dll", *ESENTPRF, &log)?;
        } else {
            log("esentprf.dll already exists, skipping.".into());
        }

        log("Conjuring uninstaller.".into());
        write_bytes(path, UNINSTALLER_EXE_NAME, UNINSTALLER_EXE, &log)?;
        progress();



        Ok(())
    }

    pub fn uninstaller_path(install_root: &Path) -> std::path::PathBuf {
        install_root.join(UNINSTALLER_EXE_NAME)
    }
}

fn write_bytes(
    parent_path: &Path,
    name: &str,
    bytes: &[u8],
    log: impl Fn(String),
) -> Result<(), TachyonInstallerError> {
    log(format!(
        "{}\\{}",
        parent_path.to_string_lossy(),
        name
    ));
    let mut file = File::create(parent_path.join(name))?;
    file.write_all(bytes)?;
    Ok(())
}
