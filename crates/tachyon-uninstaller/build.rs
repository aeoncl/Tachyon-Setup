fn main() {
    println!("cargo:rerun-if-changed=tachyon_uninstaller.rc");
    println!("cargo:rerun-if-changed=tachyon_uninstaller.exe.manifest");
    embed_resource::compile("tachyon_uninstaller.rc", embed_resource::NONE);
}
