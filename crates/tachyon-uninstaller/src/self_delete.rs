use std::env;
use std::io;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::os::windows::ffi::OsStrExt;
use winapi::um::winbase::MoveFileExW;
use winapi::um::winbase::MOVEFILE_DELAY_UNTIL_REBOOT;

pub fn relaunch_as_temp_for_self_delete() -> io::Result<()> {
    let current_exe = env::current_exe()?;

    if is_in_temp(&current_exe) {
        return Ok(());
    }

    let temp_dir = env::temp_dir();
    let temp_exe = temp_dir.join(format!(
        "tachyon_uninstaller_cleanup_{}.exe",
        std::process::id()
    ));

    std::fs::copy(&current_exe, &temp_exe)?;

    Command::new(&temp_exe)
        .arg("--cleanup")
        .arg(&current_exe)
        .spawn()?;

    Ok(())
}

pub fn delete_original_and_self(original: &Path) -> io::Result<()> {
    for _ in 0..50 {
        if std::fs::remove_file(original).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    if let Ok(self_path) = env::current_exe() {
        schedule_reboot_delete(&self_path);
    }
    Ok(())
}

fn is_in_temp(p: &Path) -> bool {
    let temp = env::temp_dir();
    p.starts_with(&temp)
}

fn schedule_reboot_delete(path: &Path) {
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MoveFileExW(wide.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT);
    }
}

