use std::path::{self, Path};

use nwd::{NwgUi, NwgPartial};
use nwg::NativeUi;

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwg::stretch::{
    geometry::{Size, Rect},
    style::{Dimension as D, FlexDirection, AlignSelf}
};
use registry::{Hive, Security};

#[derive(Default, NwgUi)]
pub struct TachyonSetup {



    #[nwg_control(size: (650, 550), position: (300, 300), title: "Tachyon Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnInit: [TachyonSetup::on_init(RC_SELF)] )]
    window: nwg::Window,

    #[nwg_control(flags:"VISIBLE", parent: window, size: (650, 450))]
    frame: nwg::Frame,

    #[nwg_partial(parent: frame)]

    #[nwg_events( (browseBtn, OnButtonClick): [TachyonSetup::browse(RC_SELF)] )]
    partial: PathSelectionPage,

    #[nwg_control(flags: "NONE", parent: window)]
    frame2: nwg::Frame,

    #[nwg_partial(parent: frame2)]
    partial2: Page2,


    #[nwg_control(text: "Next", size: (280, 50), position: (0, 500))]
    #[nwg_events( OnButtonClick: [TachyonSetup::next_page(RC_SELF)] )]
    hello_button: nwg::Button
}

impl TachyonSetup {

    fn on_init(&self) {

        let contact_dll_path = Hive::ClassesRoot.open("WOW6432Node\\CLSID\\{5FCAA434-4EB1-4BEA-B64D-51917E233068}\\InprocServer32", Security::Read)
        .or(Hive::ClassesRoot.open("CLSID\\{5FCAA434-4EB1-4BEA-B64D-51917E233068}\\InprocServer32", Security::Read));

        if let Ok(contact_dll_path) = contact_dll_path {

            if let Ok(pathData) = contact_dll_path.value("") {
                let pathAsString = pathData.to_string();
                let path = Path::new(pathAsString.as_str());
                
                self.partial.pathLabel.set_text(path.parent().unwrap_or(Path::new(".")).parent().unwrap_or(Path::new(".")).to_str().unwrap_or_default());
                self.partial.label3.set_text("Windows Live installation folder found ! :P")
            }
        }

    }

    fn browse(&self) {
        self.partial.dialog.set_default_folder(self.partial.pathLabel.text().as_str());
        let test = self.partial.dialog.run(Some(&self.window));
    }

    fn next_page(&self) {
        self.frame.set_visible(false);
        self.frame2.set_visible(true);
    }
    
  

}

#[derive(Default, NwgPartial)]
pub struct BrowserPartial {


}


#[derive(Default, NwgPartial)]
pub struct PathSelectionPage {
    #[nwg_layout(flex_direction: FlexDirection::Column, max_size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    layout: nwg::FlexboxLayout,

    #[nwg_resource(family: "Segoe UI", size: 28)]
    title_font: nwg::Font,

    #[nwg_resource(family: "Segoe UI", size: 20)]
    desc_font: nwg::Font,


    #[nwg_control(text: "Welcome to Tachyon Setup for WLM 2009", size:(650, 100), font: Some(&data.title_font) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    label1: nwg::Label,

    #[nwg_control(text: "Please select your Windows Live installation folder...", font: Some(&data.desc_font) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    label2: nwg::Label,

    #[nwg_control(text: "", font: Some(&data.desc_font) )]
    #[nwg_layout_item(layout: layout, size: Size{ width: D::Points(650.0), height: D::Points(100.0)})]
    label3: nwg::Label,

    #[nwg_layout(flex_direction: FlexDirection::Row, align_items: stretch::style::AlignItems::Center, max_size: Size{ width: D::Points(650.0), height: D::Points(300.0)})]
    layout2: nwg::FlexboxLayout,

    
    #[nwg_resource(title: "Open Messenger Folder", action: nwg::FileDialogAction::OpenDirectory)]
    dialog: nwg::FileDialog,

    #[nwg_control(text: "BLABLABLA", readonly: true)]
    #[nwg_layout_item(layout: layout2, size: Size{ width: D::Points(540.0), height: D::Auto})]
    pathLabel: nwg::TextInput,


    #[nwg_control(text: "Browse")]
    #[nwg_layout_item(layout: layout2, size: Size{ width: D::Points(90.0), height: D::Points(30.0)})]
    browseBtn: nwg::Button



}

impl PathSelectionPage {

}

#[derive(Default, NwgPartial)]
pub struct Page2 {
    #[nwg_layout(max_size: [650, 400])]
    layout: nwg::GridLayout,

    #[nwg_control(text: "Byebye", h_align: HTextAlign::Left)]
    #[nwg_layout_item(layout: layout, col: 0, row: 0)]
    label1: nwg::Label,

}

impl Page2 {

}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    
    let _app = TachyonSetup::build_ui(Default::default()).expect("Failed to build UI");
    
    nwg::dispatch_thread_events();
    
}
