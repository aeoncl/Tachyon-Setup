#![windows_subsystem = "windows"]

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

mod self_delete;
mod ui;
mod worker;

use std::env;
use std::sync::atomic::Ordering;

use nwg::NativeUi;
use ui::{TachyonUninstaller, UNINSTALL_SUCCEEDED};

fn main() {
    let args: Vec<String> = env::args().collect();

    // If launched with --cleanup <original>, we're running as the temp copy.
    if let Some(pos) = args.iter().position(|a| a == "--cleanup") {
        if let Some(orig) = args.get(pos + 1) {
            let _ = self_delete::delete_original_and_self(std::path::Path::new(orig));
            return;
        }
    }

    // Silent uninstall (for QuietUninstallString = "...\uninstall.exe /S")
    let silent = args
        .iter()
        .any(|a| a.eq_ignore_ascii_case("/S") || a == "--silent");

    if silent {
        let code = ui::run_silent();
        if code == 0 {
            let _ = self_delete::relaunch_as_temp_for_self_delete();
        }
        std::process::exit(code);
    }

    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").ok();

    let _app = TachyonUninstaller::build_ui(Default::default())
        .expect("Failed to build uninstaller UI");

    nwg::dispatch_thread_events();

    if UNINSTALL_SUCCEEDED.load(Ordering::SeqCst) {
        let _ = self_delete::relaunch_as_temp_for_self_delete();
    }
}
