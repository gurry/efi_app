# efi_app

A sample UEFI application written in Rust using the [`efi`](https://github.com/gurry/efi) crate.

## Building

To build follow the below steps:

1. Unless already installed, install `cargo-xbuild` by running `cargo install cargo-xbuild`.
4. Execute the build by running `cargo xbuild --target x86_64-unknown-uefi`
5. When the build complete the resulting EFI application `efi_app.efi` will be found in `target\x86_64-unknown-uefi\debug\`. Load this up in qemu and run it via EFI shell. You can find an OVMF binary for your use in the directory `tools` of this repo.

Build steps have been tested only on Windows. Building on non-Windows OS's might work. Currently only x64 architecture is supported.
