use std::path::{Path, PathBuf};
use registry::{Data, Hive, Security};
use thiserror::__private::AsDisplay;
use utfx::U16CString;
use crate::error::TachyonInstallerError;

pub struct RegistryService {}

const CONTACT_PROXY_CLSID: &str = "{D86BCC3A-303F-41C9-AF6B-5E30C38FAF36}";

impl RegistryService {

    pub fn install(install_path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        Self::write_identity_crl_registry_keys(&log)?;

        let contacts_path = install_path.join("Contacts");
        Self::write_contact_com_proxy_registry_keys(contacts_path.as_path(), &log)?;
        Ok(())
    }

    pub fn uninstall(log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        Self::remove_identity_crl_registry_keys(&log)?;
        Self::remove_contact_com_proxy_registry_keys(&log)?;
        Ok(())
    }

    pub fn find_installation_path() -> Result<PathBuf, TachyonInstallerError> {
        //return Err(TachyonInstallerError::PathNotExist("Failed to find installation path".to_string()));
        let contact_dll_path = Hive::ClassesRoot
            .open(
                "WOW6432Node\\CLSID\\{5FCAA434-4EB1-4BEA-B64D-51917E233068}\\InprocServer32",
                Security::Read,
            )
            .or(Hive::ClassesRoot.open(
                "CLSID\\{5FCAA434-4EB1-4BEA-B64D-51917E233068}\\InprocServer32",
                Security::Read,
            ))?;

            let path_data = contact_dll_path.value("")?;
            let path_as_string = path_data.to_string();
            let path = PathBuf::from(path_as_string);
            let contacts_folder = path.parent().ok_or(TachyonInstallerError::PathNotExist(format!("Invalid Contacts Path from Registry: {}", &path.as_display())))?;
            let msn_msgr_install_folder = contacts_folder.parent().ok_or(TachyonInstallerError::PathNotExist(format!("Invalid WLM Install Path from Registry: {}", &contacts_folder.as_display())))?;
        Ok(msn_msgr_install_folder.to_path_buf())
    }

    pub fn write_identity_crl_registry_keys(log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Creating Tachyon IdentityCRL environment.".into());

        let idcrl_env_key = Hive::LocalMachine
            .open(
                "SOFTWARE\\WOW6432Node\\Microsoft\\IdentityCRL\\Environment",
                Security::AllAccess,
            )
            .or(Hive::LocalMachine.open(
                "SOFTWARE\\Microsoft\\IdentityCRL\\Environment",
                Security::AllAccess,
            ))?;

        let tachyon_env_key = idcrl_env_key.create("Tachyon", Security::AllAccess)?;
        tachyon_env_key.set_value(
            "RemoteFile",
            &registry::Data::String(
                U16CString::from_str("http://clientconfig.passport.net/PPCRLconfig.srf").unwrap(),
            ),
        )?;
        tachyon_env_key.set_value(
            "RemoteFileLink",
            &registry::Data::String(
                U16CString::from_str("https://go.microsoft.com/fwlink/?LinkId=859524").unwrap(),
            ),
        )?;

        Ok(())
    }

    pub fn remove_identity_crl_registry_keys(log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Removing Tachyon IdentityCRL environment.".into());

        let idcrl_env_key = Hive::LocalMachine
            .open(
                "SOFTWARE\\WOW6432Node\\Microsoft\\IdentityCRL\\Environment",
                Security::AllAccess,
            )
            .or(Hive::LocalMachine.open(
                "SOFTWARE\\Microsoft\\IdentityCRL\\Environment",
                Security::AllAccess,
            ))?;

        if let Ok(key) = idcrl_env_key.open("Tachyon", Security::AllAccess) {
            key.delete_self(false)?;
        } else {
            log("Tachyon IdentityCRL environment key not found. Skipping...".into());
        }

        Ok(())
    }

    pub fn write_contact_com_proxy_registry_keys(contact_path: &Path, log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Registering Windows Live Contacts Tachyon COM proxy".into());

        let clsid_path = Hive::ClassesRoot
            .open(
                "WOW6432Node\\CLSID",
                Security::AllAccess,
            )
            .or(Hive::ClassesRoot.open(
                "CLSID",
                Security::AllAccess,
            )).unwrap();

        let draal_path = contact_path.join("draal.exe").to_str().expect("to be valid path").to_string();

        let proxy_clsid = clsid_path.create(CONTACT_PROXY_CLSID, Security::AllAccess)?;
        proxy_clsid.set_value("",  &Data::String(U16CString::from_str("Windows Live Contact Database").unwrap()))?;
        proxy_clsid.set_value("AppId",  &Data::String(U16CString::from_str(CONTACT_PROXY_CLSID).unwrap()))?;

        let local_server = proxy_clsid.create("LocalServer32", Security::AllAccess)?;
        local_server.set_value("",  &Data::String(U16CString::from_str(&draal_path).unwrap()))?;
        local_server.set_value("ServerExecutable",  &Data::String(U16CString::from_str(&draal_path).unwrap()))?;

        Ok(())
    }

    pub fn  remove_contact_com_proxy_registry_keys(log: impl Fn(String)) -> Result<(), TachyonInstallerError> {
        log("Removing Windows Live Contacts Tachyon COM proxy".into());
        let clsid_path = Hive::ClassesRoot
            .open(
                "WOW6432Node\\CLSID",
                Security::AllAccess,
            )
            .or(Hive::ClassesRoot.open(
                "CLSID",
                Security::AllAccess,
            ))?;

        if let Ok(key) = clsid_path.open(CONTACT_PROXY_CLSID, Security::AllAccess) {
            key.delete_self(true)?;
        } else {
            log("Windows Live Contacts Tachyon COM proxy not found. Skipping...".into());
        }

        Ok(())
    }


}