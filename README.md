# efi_app

A sample UEFI application written in Rust.

## Building

This crate can build only on Windows. Building on non-Windows OS's can be done, but will require a lot of manual effort. Also currently on x64 build has been tested. It may not build for other CPU architectures (like x86).

To build follow the below steps:

1. Open Visual Studio Command Prompt in this directory. Make sure you use x64 version of VS Command Prompt.
2. Unless already installed, install `xargo` by running `cargo install xargo`.
3. Create an environment variable `RUST_TARGET_PATH` and set it to the absolute path of this directory by running `set RUST_TARGET_PATH=<abs path to this dir>`. This step is needed only due to a bug in `xargo` which prevents it from correctly locating our target file `x86_64-unknown-efi.json`. When the bug is fixed, we won't need this step.
4. Execute the build by running `xargo build --target x86_64-unknown-efi`
5. When the build complete the resulting EFI application `efi_app.efi` will be found in `target\x86_64-unknown-efi\debug\`. Load this up in qemu and run it via EFI shell. You can find an OVMF binary for your use in the directory `tools` of this repo.

