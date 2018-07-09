#![no_std]
#![feature(intrinsics)]
#![feature(asm)]
#![feature(lang_items)]
#![feature(link_args)]
#![feature(compiler_builtins_lib)]
#![feature(alloc)]
#![feature(nll)]

#[allow(unused_attributes)] // The below attribute is needed to specify the entry point. Hence suppressing the warning
#[link_args = "/ENTRY:efi_start"]
extern "C" {}

extern crate rlibc;
extern crate compiler_builtins;
#[macro_use] extern crate efi;
#[macro_use] extern crate alloc;

use efi::ffi;
use efi::{
    SystemTable,
    net,
    init_env,
    io::{self, Read, BufRead},
    EfiErrorKind,
};

use alloc::String;
use core::str;


// EFI entry point. This function is the one that the UEFI platform calls when this image is loaded.
#[no_mangle]
pub extern "win64" fn efi_start(image_handle: ffi::EFI_HANDLE,
                                sys_table : *const ffi::EFI_SYSTEM_TABLE) -> isize {

    init_env(image_handle, sys_table);
    let mut sys_table = SystemTable::new(sys_table).expect("Failed to initialize system table");

    if let Err(msg) = run(&mut sys_table) {
        println!("Exiting: {}", msg);
    };

    0
}

fn run(_sys_table: &mut SystemTable) -> Result<(), String> {
    println!("Hello from UEFI");
    println!("");

    if net::dhcp::cached_dhcp_config().unwrap_or(None).is_none() { // If there's cached config then DHCP has already happend. Otherwise we start it.
        println!("Performing DHCP...");
        let dhcp_config = net::dhcp::run_dhcp().map_err(|e| format!("Dhcp failed - {}", e))?;

        println!("    Your IP: {}, Subnet mask: {}", dhcp_config.ip(), dhcp_config.subnet_mask());
        if let Some(server_ip) =  dhcp_config.dhcp_server_ip() {
            println!("    Server IP: {}", server_ip);
        }
    }

    println!("");
    println!("Testing TCP by sending HTTP request to the given addr");

    print!("Enter addr to connect to (<host>:<port>): ");
    let stdin = efi::stdin();
    let addr = stdin.lines().next().unwrap().unwrap();

    println!("Connecting to {}...", addr);

    net::TcpStream::connect(addr)
        .and_then(|mut stream| {
            println!("Connected!");
            let buf = "GET / HTTP/1.1".as_bytes();
            use io::Write;

            stream.write(&buf).unwrap();
            stream.write("\r\n".as_bytes()).unwrap();
            stream.write("Content-Length: 0\r\n".as_bytes()).unwrap();
            stream.write("\r\n".as_bytes()).unwrap();

            println!("Req sent");

            println!("");
            println!("Received resp: ");
            let mut rbuf = [0_u8; 2048];

            let read = stream.read(&mut rbuf).unwrap();

            if read == 0 {
                return Err(EfiErrorKind::NoResponse.into())
            }

            let resp = String::from_utf8_lossy(&rbuf[..read]).into_owned();

            println!("{}", resp);

            println!("");

            Ok(())
        })
        .or_else(|e| {
            Err(format!("Failed to connect. Status code: {:?}", e))
        })?;

    Ok(())
}

//---- The below code is required to make Rust compiler happy. Without it compilation will fail. ---- //
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
pub unsafe fn __chkstk() {
}

// Must have a main to satisfy rustc
fn main() {
}

// Must have a start as well to satisfy rustc
#[lang = "start"]
fn start(_main: *const u8, _argc: isize, _argv: *const *const u8) -> isize {
    0
}