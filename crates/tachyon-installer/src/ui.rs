use std::cell::RefCell;
use std::path::PathBuf;

use lazy_static_include::lazy_static_include_bytes;
use nwd::{NwgPartial, NwgUi};
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{Dimension as D, FlexDirection},
};
use nwg::Font;
use setup_core::{FileService, ProcessService, RegistryService, TachyonInstallerError};
use winapi::shared::minwindef::LRESULT;
use winapi::shared::windef::HBRUSH;
use winapi::um::wingdi::{CreateSolidBrush, SetBkColor, SetBkMode, RGB, TRANSPARENT};
use winapi::um::winuser::{
    FillRect, GetClientRect, RedrawWindow, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE,
    WM_CTLCOLORSTATIC, WM_ERASEBKGND,
};

use crate::install_service::InstallerFileService;
use crate::worker::{InstallMessage, Reporter};

lazy_static_include_bytes! {
    TACHYON_BANNER => "./img/tachyon_banner.bmp",
}

thread_local! {
    static WHITE_BRUSH: std::cell::Cell<HBRUSH> = std::cell::Cell::new(std::ptr::null_mut());
    static GRAY_BRUSH: std::cell::Cell<HBRUSH> = std::cell::Cell::new(std::ptr::null_mut());
    static FRAME_HANDLERS: RefCell<Vec<nwg::RawEventHandler>> = RefCell::new(Vec::new());
}

const PT_20: D = D::Points(20.0);
const PADDING: Rect<D> = Rect { start: PT_20, end: PT_20, top: PT_20, bottom: PT_20 };

const MARGIN_TOP_20: Rect<D> = Rect {
    start: D::Points(0.0),
    end: D::Points(0.0),
    top: PT_20,
    bottom: D::Points(0.0),
};
const MARGIN_TOP_40: Rect<D> = Rect {
    start: D::Points(0.0),
    end: D::Points(0.0),
    top: D::Points(40.0),
    bottom: D::Points(0.0),
};

#[derive(Default, NwgUi)]
pub struct TachyonSetup {
    #[nwg_control(size: (650, 500), position: (300, 300), title: "Tachyon Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [TachyonSetup::on_init(RC_SELF)], OnWindowClose: [TachyonSetup::on_window_close] )]
    window: nwg::Window,

    current_page: std::cell::Cell<u8>,

    #[nwg_control]
    #[nwg_events(OnNotice: [TachyonSetup::on_worker_message(SELF)])]
    install_notice: nwg::Notice,

    install_receiver: RefCell<Option<std::sync::mpsc::Receiver<InstallMessage>>>,

    #[nwg_resource(source_bin: Some(&*TACHYON_BANNER))]
    banner: nwg::Bitmap,

    #[nwg_control(size: (160, 450), position: (0, 0), parent: window, bitmap: Some(&data.banner))]
    sidebar: nwg::ImageFrame,

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

    #[nwg_control(text: "Back", size: (100, 30), position: (320, 460), enabled: false)]
    #[nwg_events( OnButtonClick: [TachyonSetup::back(RC_SELF)] )]
    back_button: nwg::Button,

    #[nwg_control(text: "Next", size: (100, 30), position: (420, 460))]
    #[nwg_events( OnButtonClick: [TachyonSetup::next_page(RC_SELF)] )]
    next_button: nwg::Button,

    #[nwg_control(text: "Cancel", size: (100, 30), position: (540, 460))]
    #[nwg_events( OnButtonClick: [TachyonSetup::cancel(RC_SELF)] )]
    cancel_button: nwg::Button,
}

impl TachyonSetup {
    fn on_window_close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn paint_background_colors(&self) {
        let white = ensure_brush(&WHITE_BRUSH, (255, 255, 255));
        let gray = ensure_brush(&GRAY_BRUSH, (229, 229, 229));

        paint_hwnd_color(&self.window.handle, 0x10001, gray, 229, 229, 229);
        paint_hwnd_color(&self.welcome_frame.handle, 0x10002, white, 255, 255, 255);
        paint_hwnd_color(&self.path_selection_frame.handle, 0x10003, white, 255, 255, 255);
        paint_hwnd_color(&self.progress_frame.handle, 0x10004, white, 255, 255, 255);

        unsafe {
            let hwnds = [
                self.window.handle.hwnd().unwrap(),
                self.welcome_frame.handle.hwnd().unwrap(),
                self.path_selection_frame.handle.hwnd().unwrap(),
                self.progress_frame.handle.hwnd().unwrap(),
            ];
            for h in hwnds {
                RedrawWindow(
                    h,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN,
                );
            }
        }
    }

    fn on_init(&self) {
        self.paint_background_colors();

        match RegistryService::find_installation_path() {
            Err(_) => {
                self.path_selection_page
                    .desc
                    .set_text("Could not detect Windows Live Messenger Installation folder.");
            }
            Ok(install_path) => match FileService::is_valid_install_folder(&install_path) {
                Ok(true) => {
                    self.path_selection_page.path_label.set_text(
                        install_path
                            .to_str()
                            .expect("Path to be valid at this point"),
                    );
                    self.path_selection_page
                        .desc
                        .set_text("Found Windows Live Messenger 2009 folder:");
                    self.path_selection_page.next_label.set_visible(true);
                }
                _ => {
                    self.path_selection_page.desc.set_text(
                        "Found invalid Windows Live Messenger installation folder. Please reinstall Windows Live Messenger 14 and try again.",
                    );
                }
            },
        }
    }

    fn cancel(&self) {
        nwg::stop_thread_dispatch();
    }

    fn back(&self) {
        match self.current_page.get() {
            1 => {
                self.back_button.set_enabled(false);
                self.welcome_frame.set_visible(true);
                self.path_selection_frame.set_visible(false);
                self.current_page.set(0);
            }
            _ => {}
        }
    }

    fn next_page(&self) {
        match self.current_page.get() {
            0 => {
                self.back_button.set_enabled(true);
                self.welcome_frame.set_visible(false);
                self.path_selection_frame.set_visible(true);
                self.current_page.set(1);
            }
            1 => {
                if let Ok(processes) = ProcessService::get_blocking_running_processes() {
                    if !processes.is_empty() {
                        let mut message = "Oof ! Windows Live Messenger is currently running.\r\nPlease close it before continuing.\r\n\r\nThe following processes must be stopped during setup:\n\n".to_string();
                        for process in processes {
                            message.push_str(&format!("- {}\n", process));
                        }
                        nwg::modal_info_message(
                            &self.window,
                            "Windows Live Messenger is running",
                            &message,
                        );
                        return;
                    }
                }

                self.path_selection_frame.set_visible(false);
                self.progress_frame.set_visible(true);
                self.current_page.set(2);
                self.back_button.set_enabled(false);
                self.next_button.set_enabled(false);
                self.cancel_button.set_enabled(false);
                self.start_install_task();
            }
            2 => {
                nwg::stop_thread_dispatch();
            }
            _ => {}
        }
    }

    fn start_install_task(&self) {
        let wl_install_folder_path: PathBuf =
            self.path_selection_page.path_label.text().into();
        let (tx, rx) = std::sync::mpsc::channel::<InstallMessage>();
        *self.install_receiver.borrow_mut() = Some(rx);

        let reporter = Reporter::new(tx.clone(), self.install_notice.sender());
        let sender = self.install_notice.sender();

        std::thread::spawn(move || {
            let result = do_stuff_worker(&wl_install_folder_path, &reporter);

            match result {
                Ok(()) => {
                    let _ = tx.send(InstallMessage::Done);
                }
                Err(e) => {
                    let _ = tx.send(InstallMessage::Error(format!("{}", e)));
                }
            }
            sender.notice();
        });
    }

    fn on_worker_message(&self) {
        let mut finished = false;

        {
            let receiver = self.install_receiver.borrow();
            let rx = match receiver.as_ref() {
                Some(rx) => rx,
                None => return,
            };

            while let Ok(msg) = rx.try_recv() {
                match msg {
                    InstallMessage::Log(s) => {
                        self.progress_page.logs.appendln(&s);
                    }
                    InstallMessage::Progress => {
                        self.progress_page.progress_bar.advance();
                    }
                    InstallMessage::ProgressToEnd => {
                        let range = self.progress_page.progress_bar.range();
                        self.progress_page.progress_bar.advance_delta(range.end);
                    }
                    InstallMessage::Done => {
                        self.progress_page.logs.appendln("Installation complete.");
                        self.progress_page
                            .title
                            .set_text("Installation complete");
                        self.next_button.set_text("Finish");
                        self.next_button.set_enabled(true);
                        self.cancel_button.set_enabled(true);
                        finished = true;
                    }
                    InstallMessage::Error(e) => {
                        self.progress_page.logs.appendln(&format!(
                            "An error has occured while installing:\r\n{}",
                            e
                        ));
                        self.next_button.set_text("Close");
                        self.next_button.set_enabled(true);
                        self.cancel_button.set_enabled(true);
                        finished = true;
                    }
                }
            }
        }

        if finished {
            *self.install_receiver.borrow_mut() = None;
        }
    }
}

/// Run the full install sequence on a worker thread.
/// Free function (not `&self`) so nothing non-Send sneaks in.
fn do_stuff_worker(
    wl_install_folder_path: &PathBuf,
    reporter: &Reporter,
) -> Result<(), TachyonInstallerError> {
    let log_fn = |msg| reporter.log(msg);
    let progress_fn = || reporter.progress();

    if FileService::is_installed(wl_install_folder_path) {
        reporter.log("Found older install. Cleaning up...".into());
        let _ = FileService::uninstall(wl_install_folder_path, log_fn);
        let _ = RegistryService::uninstall(log_fn);
        let _ = RegistryService::remove_uninstall_entry(log_fn);
    }

    let rollback_and_fail = |e: TachyonInstallerError| -> TachyonInstallerError {
        reporter.log(format!("Install failed: {}. Rolling back...", e));
        let _ = FileService::uninstall(wl_install_folder_path, log_fn);
        let _ = RegistryService::uninstall(log_fn);
        let _ = RegistryService::remove_uninstall_entry(log_fn);
        e
    };

    reporter.log("Installing Tachyon...".into());
    InstallerFileService::install(wl_install_folder_path, log_fn, progress_fn)
        .map_err(rollback_and_fail)?;
    RegistryService::install(wl_install_folder_path, log_fn, progress_fn)
        .map_err(rollback_and_fail)?;
    
    let _= FileService::create_start_menu_shortcut(wl_install_folder_path, log_fn);

    let uninstaller_exe = InstallerFileService::uninstaller_path(wl_install_folder_path);
    RegistryService::create_uninstall_entry(wl_install_folder_path, &uninstaller_exe, log_fn)
        .map_err(rollback_and_fail)?;

    reporter.log("Stalling for a bit so we pretend that we are a serious installer.".into());
    reporter.progress();
    std::thread::sleep(std::time::Duration::from_secs(1));

    reporter.log("Reticulating message splines.".into());
    reporter.progress();
    std::thread::sleep(std::time::Duration::from_secs(2));

    reporter.log("Unzuckerberging your dms.".into());
    std::thread::sleep(std::time::Duration::from_secs(2));

    reporter.log("Advancing the bar to the end for no reason...".into());
    reporter.progress_to_end();

    Ok(())
}

#[derive(Default, NwgPartial)]
pub struct WelcomePage {
    #[nwg_layout(flex_direction: FlexDirection::Column, padding: PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Welcome to the Tachyon Setup Wizard", font: Some(&title_font()))]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    title: nwg::Label,

    #[nwg_control(text: "This wizard will install Tachyon on your computer.\r\n\r\nTachyon is a compatibility portal that turns Windows Live \r\nMessenger into a Matrix client.\r\n\r\nA valid install of Windows Live Messenger 2009 (14.0) is required.\r\n\r\nTo continue, click Next.", font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_20,  size: Size{ width: D::Points(450.0), height: D::Points(180.0)})]
    desc: nwg::Label,
}

#[derive(Default, NwgPartial)]
pub struct PathSelectionPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, padding: PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Install Location", font: Some(&title_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    title: nwg::Label,

    #[nwg_control(text: "", font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_20, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    desc: nwg::Label,

    #[nwg_control(text: "", readonly: true, background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_40, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    path_label: nwg::TextInput,

    #[nwg_control(flags: "NONE", text: "To proceed with the install, click Next.", font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_20, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    next_label: nwg::Label,
}

#[derive(Default, NwgPartial)]
pub struct ProgressPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, padding: PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Install in progress", font: Some(&title_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    title: nwg::Label,

    #[nwg_control(step: 10, range: 0..80)]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_20, size: Size{ width: D::Points(450.0), height: D::Points(25.0)})]
    progress_bar: nwg::ProgressBar,

    #[nwg_control(text: "Status:", font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_40, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    status: nwg::Label,

    #[nwg_control(text: "", readonly: true, flags: "VISIBLE|AUTOVSCROLL|VSCROLL")]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(250.0)})]
    logs: nwg::TextBox,
}

// ---- helpers ------------------------------------------------------

fn ensure_brush(
    cell: &'static std::thread::LocalKey<std::cell::Cell<HBRUSH>>,
    rgb: (u8, u8, u8),
) -> HBRUSH {
    cell.with(|b| {
        let mut h = b.get();
        if h.is_null() {
            h = unsafe { CreateSolidBrush(RGB(rgb.0, rgb.1, rgb.2)) };
            b.set(h);
        }
        h
    })
}

fn paint_hwnd_color(
    handle: &nwg::ControlHandle,
    handler_id: usize,
    brush: HBRUSH,
    r: u8,
    g: u8,
    b: u8,
) {
    let handler = nwg::bind_raw_event_handler(handle, handler_id, move |hwnd, msg, w, _l| unsafe {
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
        None
    })
    .expect("Failed to bind raw handler");

    FRAME_HANDLERS.with(|h| h.borrow_mut().push(handler));
}

fn title_font() -> Font {
    let mut font = Font::default();
    nwg::FontBuilder::new()
        .family("Segoe UI")
        .size(28)
        .build(&mut font)
        .expect("Failed to build title font");
    font
}

fn desc_font() -> Font {
    let mut font = Font::default();
    let _ = nwg::FontBuilder::new()
        .family("Segoe UI")
        .size(20)
        .build(&mut font);
    font
}
