use crate::error::TachyonInstallerError;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

const BLOCKING_PROCESSES: &[&str] = &["msnmsgr.exe", "tachyon.exe", "wlcomm.exe"];

pub struct ProcessService;

impl ProcessService {
    fn check_if_process_running(
        process_names: &[&str],
    ) -> Result<Vec<String>, TachyonInstallerError> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(TachyonInstallerError::EnumerateProcess);
            }

            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            let mut found: Vec<String> = Vec::new();

            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]);

                    if let Some(m) = process_names.iter().find(|t| t.eq_ignore_ascii_case(&name)) {
                        found.push((*m).to_string());

                        if found.len() >= process_names.len() {
                            break;
                        }
                    }

                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
            Ok(found)
        }
    }

    pub fn get_blocking_running_processes() -> Result<Vec<String>, TachyonInstallerError> {
        Self::check_if_process_running(BLOCKING_PROCESSES)
    }
}
