#![windows_subsystem = "windows"]

use std::fs::File;
use std::io::Write;
use std::path::{self, Path, PathBuf};
use lazy_static::lazy_static;
use lazy_static_include::lazy_static_include_bytes;
use error::TachyonInstallerError;
use nwd::{NwgPartial, NwgUi};
use nwg::{Font, NativeUi};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use nwg::stretch::{
    geometry::{Rect, Size},
    style::{AlignSelf, Dimension as D, FlexDirection},
};
use registry::{Data, Hive, Security};

mod error;
mod file_service;
mod registry_service;

use utfx::U16CString;
use winapi::shared::windef::RECT;
use winapi::um::winuser::GetWindowRect;
use crate::file_service::FileService;
use crate::registry_service::RegistryService;

#[derive(Default, NwgUi)]
pub struct TachyonSetup {
    #[nwg_control(size: (650, 550), position: (300, 300), title: "Tachyon Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [TachyonSetup::on_init(RC_SELF)], OnWindowClose: [TachyonSetup::on_window_close]  )]
    window: nwg::Window,

    #[nwg_control(flags:"VISIBLE", parent: window, size: (650, 450))]
    path_selection_frame: nwg::Frame,

    #[nwg_partial(parent: path_selection_frame)]
    path_selection_page: PathSelectionPage,

    #[nwg_control(flags: "NONE", parent: window, size: (650, 450))]
    progress_frame: nwg::Frame,

    #[nwg_partial(parent: progress_frame)]
    progress_page: ProgressPage,
    
    #[nwg_control(text: "Next", size: (280, 50), position: (0, 500))]
    #[nwg_events( OnButtonClick: [TachyonSetup::next_page(RC_SELF)] )]
    next_button: nwg::Button,
}

impl TachyonSetup {
    fn on_window_close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn on_init(&self) {

        match RegistryService::find_installation_path() {
            Err(_) => {
                self.path_selection_page
                    .found_label
                    .set_text("Windows Live Messenger installation folder not found.");
                
                self.path_selection_page.desc.set_text("Could not detect Windows Live Messenger Installation folder.")
            }
            Ok(install_path) => {
                match FileService::is_valid_install_folder(&install_path) {
                    Ok(true) => {
                        self.path_selection_page.path_label.set_text(
                            install_path.to_str().expect("Path to be valid at this point"),
                        );
                        self.path_selection_page
                            .found_label
                            .set_text("Windows Live installation folder auto-detected !");
                    }
                    _ => {
                        self.path_selection_page
                            .found_label
                            .set_text("Windows Live Messenger installation folder not found.");

                        self.path_selection_page.desc.set_text("Found invalid Windows Live Messenger installation folder. Please reinstall Windows Live Messenger 14 and try again.")
                    }
                }
                

            }
        }
    }
    
    fn next_page(&self) {
        self.path_selection_frame.set_visible(false);
        self.progress_frame.set_visible(true);
        self.install();
    }

    fn install(&self) {
        let wl_install_folder_path: PathBuf = self.path_selection_page.path_label.text().into();
        let log_function = |msg| {
            self.log(msg);
        };

        match self.do_stuff(&wl_install_folder_path) {
            Ok(..) => {
                self.log("All good".into());
            }
            Err(e) => {
                self.log(format!("Error: {}", e));
                let _ = FileService::uninstall(&wl_install_folder_path, log_function);
                let _ = RegistryService::uninstall(log_function);
            }
        }
    }

    fn do_stuff(&self, wl_install_folder_path: &PathBuf) -> Result<(), TachyonInstallerError> {
        let log_function = |msg| {
            self.log(msg);
        };


        if FileService::is_installed(wl_install_folder_path) {
            self.log("Found older install. Cleaning up...".into());
            let _ = FileService::uninstall(wl_install_folder_path, log_function);
            let _ = RegistryService::uninstall(log_function);
        }


        self.log("Installing new files...".into());
        FileService::install(wl_install_folder_path, log_function)?;
        RegistryService::install(wl_install_folder_path, log_function)?;


        Ok(())
    }

    fn log(&self, msg: String) {
        self.progress_page.logs.appendln(&msg);
    }
}

#[derive(Default, NwgPartial)]
pub struct BrowserPartial {}

#[derive(Default, NwgPartial)]
pub struct PathSelectionPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, max_size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Install Tachyon", size:(650, 100), font: Some(&title_font()) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    title: nwg::Label,

    #[nwg_control(text: "Please select your Windows Live installation folder...", font: Some(&desc_font()) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    desc: nwg::Label,

    #[nwg_control(text: "", font: Some(&desc_font()) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    found_label: nwg::Label,

    #[nwg_layout(flex_direction: FlexDirection::Row, align_items: stretch::style::AlignItems::Center, max_size: Size{ width: D::Points(650.0), height: D::Points(300.0)})]
    layout2: nwg::FlexboxLayout,

    #[nwg_control(text: "", readonly: true)]
    #[nwg_layout_item(layout: layout2, size: Size{ width: D::Points(540.0), height: D::Points(30.0)})]
    path_label: nwg::TextInput,

}

impl PathSelectionPage {}

#[derive(Default, NwgPartial)]
pub struct ProgressPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, max_size: Size{ width: D::Points(650.0), height: D::Points(450.0)}, justify_content: stretch::style::JustifyContent::FlexStart)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Install in progress", font: Some(&title_font()) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Auto, height: D::Points(30.0)})]
    label1: nwg::Label,

    #[nwg_control(step: 10, range: 0..100)]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(620.0), height: D::Points(25.0)})]
    progress_bar: nwg::ProgressBar,

    #[nwg_control(text: "Status:", font: Some(&desc_font()) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Auto, height: D::Points(30.0)})]
    status: nwg::Label,

    #[nwg_control(text: "", readonly: true, size: (620, 300), flags: "VISIBLE|AUTOVSCROLL" )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(620.0), height: D::Points(300.0)})]
    logs: nwg::TextBox,
}



impl ProgressPage {}

fn title_font() -> Font {
    let mut font = Font::default();
    nwg::FontBuilder::new()
        .family("Segoe UI")
        .size(28)
        .build(&mut font).expect("TODO: panic message");
    return font;
}

fn desc_font() -> Font {
    let mut font = Font::default();
    nwg::FontBuilder::new()
        .family("Segoe UI")
        .size(20)
        .build(&mut font);
    return font;
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    nwg::Font::set_global_family("Segoe UI");

    let _app = TachyonSetup::build_ui(Default::default()).expect("Failed to build UI");

    nwg::dispatch_thread_events();
}
