#![no_std]
#![feature(intrinsics)]
#![feature(asm)]
#![feature(lang_items)]
#![feature(link_args)]
#![feature(compiler_builtins_lib)]

#[allow(unused_attributes)] // The below attribute is needed to specify the entry point. Hence suppressing the warning
#[link_args = "/ENTRY:efi_start"]
extern "C" {}

extern crate rlibc;
extern crate compiler_builtins;
extern crate efi;

use efi::ffi;
use efi::{boot_services::BootServices, protocols::PxeBaseCodeProtocol, EfiSystemTable};
use core::fmt::Write;

#[no_mangle]
#[lang="panic_fmt"]
pub extern fn panic_fmt(_: ::core::fmt::Arguments, _: &'static str, _: u32) -> ! {
    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}

#[no_mangle]
pub fn abort() -> ! {
	loop {}
}

#[no_mangle]
pub fn breakpoint() -> ! {
	loop {}
}

#[no_mangle]
pub fn __chkstk() -> ! {
	loop {}
}

// Must have a main to satisfy rustc
fn main() {
}

// Must have a start as well to satisfy rustc
#[lang = "start"]
fn start(_main: *const u8, _argc: isize, _argv: *const *const u8) -> isize {
    0
}

enum Void {}

#[no_mangle]
pub extern "win64" fn efi_start(_image_handle: ffi::EFI_HANDLE,
                                sys_table : *const ffi::EFI_SYSTEM_TABLE) -> isize {
    let sys_table2 = EfiSystemTable(sys_table);
    let mut c = sys_table2.console();
    write!(c, "Hello from UEFI motherfucker\r\n");

    unsafe {
        let bs = (*sys_table).BootServices;
        let bs = BootServices::from(bs);
        let pxe_protocol = bs.locate_protocol::<PxeBaseCodeProtocol>();
        if let Err(e) = pxe_protocol {
                write!(c, "failed: {}\r\n", e);
                return 0;
        }

        let pxe_protocol = pxe_protocol.unwrap();

        write!(c, "Starting Pxe\r\n");
        pxe_protocol.start(false);

        write!(c, "Started Pxe\r\n");
        match pxe_protocol.dhcp(false) {
            Ok(_) => write!(c, "Dhcp succeeded\r\n"),
            Err(e) => write!(c, "Dhcp failed: {:?}\r\n", e),
        };
    }

    0
}