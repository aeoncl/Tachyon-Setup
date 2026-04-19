#![windows_subsystem = "windows"]

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

mod install_service;
mod ui;
mod worker;

use nwg::NativeUi;
use ui::TachyonSetup;

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").ok();

    let _app = TachyonSetup::build_ui(Default::default()).expect("Failed to build UI");

    nwg::dispatch_thread_events();
}
