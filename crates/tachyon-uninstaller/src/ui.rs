use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

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

use crate::worker::{Reporter, UninstallMessage};

lazy_static_include_bytes! {
    TACHYON_BANNER => "./img/tachyon_banner.bmp",
}

pub static UNINSTALL_SUCCEEDED: AtomicBool = AtomicBool::new(false);

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
pub struct TachyonUninstaller {
    #[nwg_control(size: (650, 500), position: (300, 300), title: "Tachyon Uninstall", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [TachyonUninstaller::on_init(RC_SELF)], OnWindowClose: [TachyonUninstaller::on_window_close] )]
    window: nwg::Window,

    current_page: std::cell::Cell<u8>,

    uninstall_in_progress: std::cell::Cell<bool>,
    install_path: RefCell<Option<PathBuf>>,

    #[nwg_control]
    #[nwg_events(OnNotice: [TachyonUninstaller::on_worker_message(SELF)])]
    uninstall_notice: nwg::Notice,

    uninstall_receiver: RefCell<Option<std::sync::mpsc::Receiver<UninstallMessage>>>,

    #[nwg_resource(source_bin: Some(&*TACHYON_BANNER))]
    banner: nwg::Bitmap,

    #[nwg_control(size: (160, 450), position: (0, 0), parent: window, bitmap: Some(&data.banner))]
    sidebar: nwg::ImageFrame,

    #[nwg_control(flags: "VISIBLE", parent: window, position: (160, 0), size: (490, 450))]
    confirm_frame: nwg::Frame,

    #[nwg_partial(parent: confirm_frame)]
    confirm_page: ConfirmPage,

    #[nwg_control(flags: "NONE", parent: window, position: (160, 0), size: (490, 450))]
    progress_frame: nwg::Frame,

    #[nwg_partial(parent: progress_frame)]
    progress_page: ProgressPage,

    #[nwg_control(text: "Uninstall", size: (100, 30), position: (420, 460))]
    #[nwg_events( OnButtonClick: [TachyonUninstaller::primary_click(RC_SELF)] )]
    primary_button: nwg::Button,

    #[nwg_control(text: "Cancel", size: (100, 30), position: (540, 460))]
    #[nwg_events( OnButtonClick: [TachyonUninstaller::cancel(RC_SELF)] )]
    cancel_button: nwg::Button,
}

impl TachyonUninstaller {
    fn on_window_close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn paint_background_colors(&self) {
        let white = ensure_brush(&WHITE_BRUSH, (255, 255, 255));
        let gray = ensure_brush(&GRAY_BRUSH, (229, 229, 229));

        paint_hwnd_color(&self.window.handle, 0x20001, gray, 229, 229, 229);
        paint_hwnd_color(&self.confirm_frame.handle, 0x20002, white, 255, 255, 255);
        paint_hwnd_color(&self.progress_frame.handle, 0x20003, white, 255, 255, 255);

        unsafe {
            let hwnds = [
                self.window.handle.hwnd().unwrap(),
                self.confirm_frame.handle.hwnd().unwrap(),
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

        match RegistryService::read_install_location_from_uninstall_entry() {
            Ok(path) => {
                self.confirm_page
                    .path_label
                    .set_text(path.to_str().unwrap_or(""));
                *self.install_path.borrow_mut() = Some(path);
            }
            Err(_) => {
                self.confirm_page
                    .desc
                    .set_text("Could not locate Tachyon install. The uninstall entry may already be gone.");
                self.primary_button.set_enabled(false);
            }
        }
    }

    fn cancel(&self) {
        if self.uninstall_in_progress.get() {
            return;
        }

        if self.current_page.get() == 0 {
            let params = nwg::MessageParams {
                title: "Cancel uninstall?",
                content: "Tachyon will remain installed.\r\n\r\nAre you sure you want to cancel?",
                buttons: nwg::MessageButtons::YesNo,
                icons: nwg::MessageIcons::Question,
            };
            if nwg::modal_message(&self.window, &params) != nwg::MessageChoice::Yes {
                return;
            }
        }

        nwg::stop_thread_dispatch();
    }

    fn primary_click(&self) {
        match self.current_page.get() {
            0 => {
                if let Ok(processes) = ProcessService::get_blocking_running_processes() {
                    if !processes.is_empty() {
                        let mut message = "Windows Live Messenger is currently running.\r\nPlease close it before continuing.\r\n\r\nThe following processes must be stopped:\n\n".to_string();
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

                self.confirm_frame.set_visible(false);
                self.progress_frame.set_visible(true);
                self.current_page.set(1);
                self.primary_button.set_enabled(false);
                self.cancel_button.set_enabled(false);
                self.uninstall_in_progress.set(true);
                self.start_uninstall_task();
            }
            1 => {
                nwg::stop_thread_dispatch();
            }
            _ => {}
        }
    }

    fn start_uninstall_task(&self) {
        let install_path = match self.install_path.borrow().clone() {
            Some(p) => p,
            None => {
                self.progress_page
                    .logs
                    .appendln("No install path known; aborting.");
                self.primary_button.set_text("Close");
                self.primary_button.set_enabled(true);
                self.cancel_button.set_enabled(true);
                self.uninstall_in_progress.set(false);
                return;
            }
        };

        let (tx, rx) = std::sync::mpsc::channel::<UninstallMessage>();
        *self.uninstall_receiver.borrow_mut() = Some(rx);

        let reporter = Reporter::new(tx.clone(), self.uninstall_notice.sender());
        let sender = self.uninstall_notice.sender();

        std::thread::spawn(move || {
            let result = do_uninstall_worker(&install_path, &reporter);
            match result {
                Ok(()) => {
                    let _ = tx.send(UninstallMessage::Done);
                }
                Err(e) => {
                    let _ = tx.send(UninstallMessage::Error(format!("{}", e)));
                }
            }
            sender.notice();
        });
    }

    fn on_worker_message(&self) {
        let mut finished = false;

        {
            let receiver = self.uninstall_receiver.borrow();
            let rx = match receiver.as_ref() {
                Some(rx) => rx,
                None => return,
            };

            while let Ok(msg) = rx.try_recv() {
                match msg {
                    UninstallMessage::Log(s) => {
                        self.progress_page.logs.appendln(&s);
                    }
                    UninstallMessage::Progress => {
                        self.progress_page.progress_bar.advance();
                    }
                    UninstallMessage::ProgressToEnd => {
                        let range = self.progress_page.progress_bar.range();
                        self.progress_page.progress_bar.advance_delta(range.end);
                    }
                    UninstallMessage::Done => {
                        self.progress_page
                            .logs
                            .appendln("Uninstall complete.");
                        self.progress_page
                            .title
                            .set_text("Uninstall complete");
                        self.primary_button.set_text("Finish");
                        self.primary_button.set_enabled(true);
                        self.cancel_button.set_enabled(true);

                        UNINSTALL_SUCCEEDED.store(true, Ordering::SeqCst);

                        self.uninstall_in_progress.set(false);
                        finished = true;
                    }
                    UninstallMessage::Error(e) => {
                        self.progress_page
                            .logs
                            .appendln(&format!("Uninstall failed:\r\n{}", e));
                        self.primary_button.set_text("Close");
                        self.primary_button.set_enabled(true);
                        self.cancel_button.set_enabled(true);
                        self.uninstall_in_progress.set(false);
                        finished = true;
                    }
                }
            }
        }

        if finished {
            *self.uninstall_receiver.borrow_mut() = None;
        }
    }
}

fn do_uninstall_worker(
    install_path: &PathBuf,
    reporter: &Reporter,
) -> Result<(), TachyonInstallerError> {
    let log_fn = |msg| reporter.log(msg);
    let progress_fn = || reporter.progress();

    reporter.log("Removing Tachyon files...".into());
    FileService::uninstall(install_path, log_fn)?;
    progress_fn();

    reporter.log("Removing registry entries...".into());
    RegistryService::uninstall(log_fn)?;
    progress_fn();

    RegistryService::remove_uninstall_entry(log_fn)?;
    progress_fn();

    reporter.progress_to_end();
    Ok(())
}

pub fn run_silent() -> i32 {
    let install_path = match RegistryService::read_install_location_from_uninstall_entry() {
        Ok(p) => p,
        Err(_) => return 1,
    };

    let noop = |_: String| {};

    if FileService::uninstall(&install_path, noop).is_err() {
        return 2;
    }
    if RegistryService::uninstall(noop).is_err() {
        return 3;
    }
    if RegistryService::remove_uninstall_entry(noop).is_err() {
        return 4;
    }
    0
}

// ---- pages --------------------------------------------------------

#[derive(Default, NwgPartial)]
pub struct ConfirmPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, padding: PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Uninstall Tachyon", font: Some(&title_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    title: nwg::Label,

    #[nwg_control(text: "This will remove Tachyon from your Windows Live Messenger install and restore the original Messenger behaviour.\r\n\r\nYour Windows Live Messenger itself will not be uninstalled.\r\n\r\nTo continue, click Uninstall.", font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_20, size: Size{ width: D::Points(450.0), height: D::Points(120.0)})]
    desc: nwg::Label,

    #[nwg_control(text: "Install location:", font: Some(&desc_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, margin: MARGIN_TOP_20, size: Size{ width: D::Points(450.0), height: D::Points(20.0)})]
    path_title: nwg::Label,

    #[nwg_control(text: "", readonly: true, background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    path_label: nwg::TextInput,
}

#[derive(Default, NwgPartial)]
pub struct ProgressPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, padding: PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Uninstall in progress", font: Some(&title_font()), background_color: Some([255, 255, 255]))]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(450.0), height: D::Points(30.0)})]
    title: nwg::Label,

    #[nwg_control(step: 1, range: 0..10)]
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
    let _ = nwg::FontBuilder::new()
        .family("Segoe UI")
        .size(28)
        .build(&mut font);
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
