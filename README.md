# Tachyon Setup

Setup wizards for Tachyon.


It composed of three rust crates:

- **setup-core**: Common utilities shared between installer and uninstaller
- **tachyon-installer**: Installer wizard. Depends on `tachyon-uninstaller` build output.
- **tachyon-uninstaller**: Uninstaller wizard

## Build instructions

1. Place all the required binaries in the `tachyon-installer/assets` directory.
   - draal.exe
   - draal_contacts.ini
   - draal_msnmsgr.ini
   - epsilon3.dll
   - zathras.dll
   - tachyon.exe
   - esent.dll
   - esentprf.dll
   

2. Run `cargo build -p tachyon-uninstaller` to produce the uninstaller binary
3. Run `cargo build -p tachyon-installer` to produce the installer binary

## Usage

1. Install Windows Live Messenger 2009 (14.0)
2. Run the installer
3. Run Windows Live Messenger (Tachyonized)