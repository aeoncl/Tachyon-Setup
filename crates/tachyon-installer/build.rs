use std::path::PathBuf;

fn main() {
    //<workspace>/crates/tachyon-installer
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to walk up to workspace root");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target = std::env::var("TARGET").unwrap_or_default();

    let target_root = workspace_root.join("target");
    let with_triple = target_root.join(&target).join(&profile);
    let without_triple = target_root.join(&profile);

    let uninstaller_exe = if with_triple.join("tachyon_uninstaller.exe").exists() {
        with_triple.join("tachyon_uninstaller.exe")
    } else {
        without_triple.join("tachyon_uninstaller.exe")
    };

    if !uninstaller_exe.exists() {
        panic!(
            "\n\n\
             Uninstaller binary not found. Checked:\n\
             \x20   {}\n\
             \x20   {}\n\n\
             Build the whole workspace first with `cargo build-all`{}.\n\
             (Defined in .cargo/config.toml — builds the uninstaller\n\
             before the installer so the binary exists to embed.)\n\n",
            with_triple.join("tachyon_uninstaller.exe").display(),
            without_triple.join("tachyon_uninstaller.exe").display(),
            if profile == "release" { "-release" } else { "" },
        );
    }

    println!(
        "cargo:rustc-env=UNINSTALLER_EXE_PATH={}",
        uninstaller_exe.display()
    );
    println!("cargo:rerun-if-changed={}", uninstaller_exe.display());
    println!("cargo:rerun-if-changed=tachyon_installer.rc");
    println!("cargo:rerun-if-changed=tachyon_installer.exe.manifest");

    embed_resource::compile("tachyon_installer.rc", embed_resource::NONE);
}
