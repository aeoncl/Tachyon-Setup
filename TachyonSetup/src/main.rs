#![windows_subsystem = "windows"]

use std::path::{self, Path, PathBuf};

use adapter::bin_patcher::{patch_bytes, patch_string, read_string};
use adapter::bin_patcher_error::BinPatcherError;
use error::TachyonInstallerError;
use nwd::{NwgPartial, NwgUi};
use nwg::{Font, NativeUi};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use nwg::stretch::{
    geometry::{Rect, Size},
    style::{AlignSelf, Dimension as D, FlexDirection},
};
use registry::{Hive, Security};

mod adapter;
mod error;
use utfx::U16CString;

#[derive(Default, NwgUi)]
pub struct TachyonSetup {
    #[nwg_control(size: (650, 550), position: (300, 300), title: "Tachyon Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [TachyonSetup::on_init(RC_SELF)], OnWindowClose: [TachyonSetup::on_window_close]  )]
    window: nwg::Window,

    #[nwg_control(flags:"VISIBLE", parent: window, size: (650, 450))]
    path_selection_frame: nwg::Frame,

    #[nwg_partial(parent: path_selection_frame)]
    #[nwg_events( (browse_btn, OnButtonClick): [TachyonSetup::browse(RC_SELF)] )]
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
        let contact_dll_path = Hive::ClassesRoot
            .open(
                "WOW6432Node\\CLSID\\{5FCAA434-4EB1-4BEA-B64D-51917E233068}\\InprocServer32",
                Security::Read,
            )
            .or(Hive::ClassesRoot.open(
                "CLSID\\{5FCAA434-4EB1-4BEA-B64D-51917E233068}\\InprocServer32",
                Security::Read,
            ));

        if let Ok(contact_dll_path) = contact_dll_path {
            if let Ok(path_data) = contact_dll_path.value("") {
                let path_as_string = path_data.to_string();
                let path = Path::new(path_as_string.as_str());

                self.path_selection_page.path_label.set_text(
                    path.parent()
                        .unwrap_or(Path::new("."))
                        .parent()
                        .unwrap_or(Path::new("."))
                        .to_str()
                        .unwrap_or_default(),
                );
                self.path_selection_page
                    .found_label
                    .set_text("Windows Live installation folder found ! :P");
                return;
            }
        }

        self.path_selection_page
            .found_label
            .set_text("Windows Live installation folder not found ! :(");
    }

    fn browse(&self) {
        self.path_selection_page
            .dialog
            .set_default_folder(self.path_selection_page.path_label.text().as_str());
        let test = self.path_selection_page.dialog.run(Some(&self.window));
    }

    fn next_page(&self) {
        self.path_selection_frame.set_visible(false);
        self.progress_frame.set_visible(true);
        self.install();
    }

    fn install(&self) {
        let wl_install_folder_path: PathBuf = self.path_selection_page.path_label.text().into();
        match self.do_stuff(&wl_install_folder_path) {
            Ok(..) => {
                self.log("All good".into());
            }
            Err(e) => {
                self.log(format!("Error: {}", e));
            }
        }
    }

    fn do_stuff(&self, wl_install_folder_path: &PathBuf) -> Result<(), TachyonInstallerError> {
        let msgr_install_folder_path = wl_install_folder_path.join("Messenger");
        let msnmsgr_exe_path = msgr_install_folder_path.join("msnmsgr.exe");

        if !msnmsgr_exe_path.is_file() {
            return Err(TachyonInstallerError::PathNotExist(
                msnmsgr_exe_path.to_string_lossy().into(),
            ));
        };

        self.idcrl(&msnmsgr_exe_path)?;

        Ok(())
    }

    fn idcrl(&self, msnmsgr_exe_path: &PathBuf) -> Result<(), TachyonInstallerError> {
        self.log("Creating Tachyon IDCRL environment".into());

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
                U16CString::from_str("http://127.0.0.1:8080/ppcrlconfig.srf").unwrap(),
            ),
        )?;
        tachyon_env_key.set_value(
            "RemoteFileLink",
            &registry::Data::String(
                U16CString::from_str("http://127.0.0.1:8080/wlidsvcconfig.xml").unwrap(),
            ),
        )?;

        let patch_addr = 0x3c750;
        let idcrl_env = read_string(msnmsgr_exe_path, patch_addr, 10)?;

        match idcrl_env.as_str() {
            "Production" => {
                self.log("Patching msnmsgr.exe idcrl environment".into());
                patch_string(msnmsgr_exe_path, 0x3c750, "Tachyon\0\0\0".into())?;
                let new_id_crl_env= read_string(msnmsgr_exe_path, 0x3c750, 10)?;
                if new_id_crl_env != "Tachyon" { return Err(BinPatcherError::UnexpectedStringPatch { binary_path: msnmsgr_exe_path.to_string_lossy().into(), address: patch_addr, expected: "Tachyon".into(), actual: new_id_crl_env })? }
            },
            "Tachyon\0\0\0" => {
                self.log("msnmsgr.exe idcrl environment was already patched, skipping...".into());
            },
            _ => {
                return Err(BinPatcherError::UnexpectedBinaryStringContent { binary_path: msnmsgr_exe_path.to_string_lossy().into(), address: patch_addr, expected: "Production".into(), actual: idcrl_env })?;
            }
        }
        return Ok(());
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

    #[nwg_control(text: "Welcome to Tachyon Setup for WLM 2009", size:(650, 100), font: Some(&title_font()) )]
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

    #[nwg_resource(title: "Open Messenger Folder", action: nwg::FileDialogAction::OpenDirectory)]
    dialog: nwg::FileDialog,

    #[nwg_control(text: "BLABLABLA", readonly: true)]
    #[nwg_layout_item(layout: layout2, size: Size{ width: D::Points(540.0), height: D::Auto})]
    path_label: nwg::TextInput,

    #[nwg_control(text: "Browse")]
    #[nwg_layout_item(layout: layout2, size: Size{ width: D::Points(90.0), height: D::Points(30.0)})]
    browse_btn: nwg::Button,
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
        .build(&mut font);
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
