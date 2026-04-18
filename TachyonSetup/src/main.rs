#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::{self, Path, PathBuf};
use lazy_static::lazy_static;
use lazy_static_include::lazy_static_include_bytes;
use error::TachyonInstallerError;
use winapi::shared::minwindef::LRESULT;
use winapi::um::winuser::{FillRect, GetClientRect, InvalidateRect, UpdateWindow,
                          WM_ERASEBKGND, WM_CTLCOLORSTATIC};
use winapi::um::winuser::{RedrawWindow, RDW_INVALIDATE, RDW_ERASE, RDW_ALLCHILDREN};

use nwd::{NwgPartial, NwgUi};
use nwg::{Font, NativeUi};
use utfx::U16CString;
use winapi::shared::windef::{HBRUSH, RECT};
use crate::file_service::FileService;
use crate::registry_service::RegistryService;

use nwg::stretch::{
    geometry::{Rect, Size},
    style::{AlignSelf, Dimension as D, FlexDirection},
};
use registry::{Data, Hive, Security};
use winapi::um::wingdi::{CreateSolidBrush, SetBkColor, SetBkMode, RGB, TRANSPARENT};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

mod error;
mod file_service;
mod registry_service;

lazy_static_include_bytes! {
    TACHYON_BANNER => "./img/tachyon_banner.bmp",
}

thread_local! {
    static WHITE_BRUSH: std::cell::Cell<HBRUSH> = std::cell::Cell::new(std::ptr::null_mut());
    static GRAY_BRUSH: std::cell::Cell<HBRUSH> = std::cell::Cell::new(std::ptr::null_mut());
    static FRAME_HANDLERS: RefCell<Vec<nwg::RawEventHandler>> = RefCell::new(Vec::new());
}

#[derive(Default, NwgUi)]
pub struct TachyonSetup {
    #[nwg_control(size: (650, 500), position: (300, 300), title: "Tachyon Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [TachyonSetup::on_init(RC_SELF)], OnWindowClose: [TachyonSetup::on_window_close]  )]
    window: nwg::Window,

    current_page: std::cell::Cell<u8>,

    // Banner — persistent across all pages
    #[nwg_resource(source_bin: Some(&*TACHYON_BANNER))]
    banner: nwg::Bitmap,

    #[nwg_control(size: (160, 450), position: (0, 0), parent: window, bitmap: Some(&data.banner))]
    sidebar: nwg::ImageFrame,

    // Frames now only cover the right-hand content area
    #[nwg_control(flags: "VISIBLE", parent: window, position: (160, 0), size: (490, 450))]
    welcome_frame: nwg::Frame,

    #[nwg_partial(parent: welcome_frame)]
    welcome_page: WelcomePage,

    #[nwg_control(flags: "NONE", parent: window, position: (160, 0), size: (490, 450))]
    path_selection_frame: nwg::Frame,

    #[nwg_partial(parent: path_selection_frame)]
    path_selection_page: PathSelectionPage,

    #[nwg_control(flags: "NONE", parent: window, position: (160, 0), size: (490, 450))]
    progress_frame: nwg::Frame,

    #[nwg_partial(parent: progress_frame)]
    progress_page: ProgressPage,

    #[nwg_control(text: "Next", size: (150, 34), position: (484, 458))]
    #[nwg_events( OnButtonClick: [TachyonSetup::next_page(RC_SELF)] )]
    next_button: nwg::Button,
}

impl TachyonSetup {
    fn on_window_close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn on_init(&self) {

        let white = ensure_brush(&WHITE_BRUSH, (255, 255, 255));
        // for the footer
        let gray  = ensure_brush(&GRAY_BRUSH,  (229, 229, 229));

        paint_hwnd_color(&self.window.handle,              0x10001, gray,  229, 229, 229);
        paint_hwnd_color(&self.welcome_frame.handle,        0x10002, white, 255, 255, 255);
        paint_hwnd_color(&self.path_selection_frame.handle, 0x10003, white, 255, 255, 255);
        paint_hwnd_color(&self.progress_frame.handle,       0x10004, white, 255, 255, 255);

        unsafe {
            let hwnds = [
                self.window.handle.hwnd().unwrap(),
                self.welcome_frame.handle.hwnd().unwrap(),
                self.path_selection_frame.handle.hwnd().unwrap(),
                self.progress_frame.handle.hwnd().unwrap(),
            ];
            for h in hwnds {
                //Force to redraw the window with the correct background color, including child components like inputs and stuff
                RedrawWindow(h, std::ptr::null(), std::ptr::null_mut(),
                             RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN);
            }
        }

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
        match self.current_page.get() {
            0 => {
                self.welcome_frame.set_visible(false);
                self.path_selection_frame.set_visible(true);
                self.current_page.set(1);
            }
            1 => {
                self.path_selection_frame.set_visible(false);
                self.progress_frame.set_visible(true);
                self.current_page.set(2);
                self.install();
            }
            _ => {}
        }
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
pub struct WelcomePage {
    #[nwg_control(text: "Welcome to the Tachyon Setup Wizard", size: (460, 150), position: (25, 20), font: Some(&title_font()), background_color: Some([255, 255, 255]))]
    title: nwg::Label,

    #[nwg_control(text: "This wizard will install Tachyon on your computer.\r\n\r\nTachyon is a compatibility portal that turns Windows Live \r\nMessenger into a Matrix client.\r\n\r\nA valid install of Windows Live Messenger 2009 (14.0) is required.\r\n\r\nTo continue, click Next.", size: (460, 280), position: (25, 190), font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    desc: nwg::Label,
}

impl WelcomePage {}

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


fn ensure_brush(cell: &'static std::thread::LocalKey<std::cell::Cell<HBRUSH>>, rgb: (u8, u8, u8)) -> HBRUSH {
    cell.with(|b| {
        let mut h = b.get();
        if h.is_null() {
            h = unsafe { CreateSolidBrush(RGB(rgb.0, rgb.1, rgb.2)) };
            b.set(h);
        }
        h
    })
}

//All this shit is just to paint the window background color.
fn paint_hwnd_color(handle: &nwg::ControlHandle, handler_id: usize, brush: HBRUSH, r: u8, g: u8, b: u8) {
    let handler = nwg::bind_raw_event_handler(handle, handler_id, move |hwnd, msg, w, _l| {
        unsafe {
            match msg {
                WM_ERASEBKGND => {
                    let hdc = w as winapi::shared::windef::HDC;
                    let mut rc = std::mem::zeroed();
                    GetClientRect(hwnd, &mut rc);
                    FillRect(hdc, &rc, brush);
                    return Some(1 as LRESULT);
                }
                WM_CTLCOLORSTATIC => {
                    let hdc = w as winapi::shared::windef::HDC;
                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetBkColor(hdc, RGB(r, g, b));
                    return Some(brush as LRESULT);
                }
                _ => {}
            }
        }
        None
    }).expect("Failed to bind raw handler");

    FRAME_HANDLERS.with(|h| h.borrow_mut().push(handler));
}

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
