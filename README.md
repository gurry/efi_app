# efi_app

A sample UEFI application written in Rust.

## Building

This crate can build only on Windows. Building on non-Windows OS's can be done, but will require a lot of manual effort.

To build follow the below steps:

1. Open Windows console in this directory.
2. Unless already installed, install `xargo` by running `cargo install xargo`.
3. Add the path to Visual Studio linker `link.exe` to the `PATH` in this console session by running `set PATH=%PATH%;<path to dir containing link.exe>`.
4. Create an environment variable `RUST_TARGET_PATH` and set it to the absolute path of this directory by running `set RUST_TARGET_PATH=<abs path to this dir>`. This step is needed only due to a bug in `xargo` which prevents it from correctly locating our target file `x86_64-unknown-efi.json`. When the bug is fixed, we won't need this step.
5. Execute the build by running `xargo build --target x86_64-unknown-efi`
6. When the build complete the resulting EFI application `efi_app.efi` will be found in `target\x86_64-unknown-efi\debug\`. Load this up in qemu and run it via EFI shell. You can find an OVMF binary for your use in the directory `tools` of this repo.

### Known Issue

You could use Visual Studio command prompt instead of the regular console to avoid having to set the path to `link.exe`. If you do, you may run into the following linker error:
  `msvcrt.lib(chkstk.obj) : fatal error LNK1112: module machine type 'x86' conflicts with target machine type 'x64'`

I haven't investigated why this issue occurs. One workaround is to open both a regular console and VS command prompt, attempt the first build in the former and then run all later builds in the latter. The above error occurs only during building the build script of `compiler-builtins` crate (which is a dependency), but the regular console should successfully build it although it fails to build this crate itself. At that point switch over to the VS command prompt and run the rest of your builds there. `compiler-builtins` build script will not need to be built again and hence you won't see the error again until the next time you run `xargo clean`.
