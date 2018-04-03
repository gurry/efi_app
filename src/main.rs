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
use efi::{
    protocols::{
        PxeBaseCodeProtocol, 
        LoadFileProtocol, 
        LoadedImageProtocol,
        BootType, 
        BOOT_LAYER_INITIAL, 
        DiscoverInfo, 
        SrvListEntry
    },
    boot_services::InterfaceType,
    SystemTable,
    net,
    init_env,
    io
};

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

#[no_mangle]
pub extern "win64" fn efi_start(image_handle: ffi::EFI_HANDLE,
                                sys_table : *const ffi::EFI_SYSTEM_TABLE) -> isize {

    init_env(image_handle, sys_table);
    let sys_table = SystemTable(sys_table);
    let mut c = sys_table.console();

    write!(c, "Hello from UEFI\r\n");

    let mut bs = sys_table.boot_services();

    write!(c, "Testing DHCP/PXE\r\n");
    // // TODO: see tianocore-edk2\NetworkPkg\UefiPxeBcDxe\PxeBcBoot.c file to know to implement PXE sequence especially the method PxeBcDiscoverBootFile
    let pxe_protocol = bs.locate_protocol::<PxeBaseCodeProtocol>();
    if let Err(e) = pxe_protocol {
            write!(c, "failed: {}\r\n", e);
            return 0;
    }

    let pxe_protocol = pxe_protocol.unwrap();

    if !pxe_protocol.mode().started() {
        write!(c, "Starting Pxe\r\n");
        pxe_protocol.start(false);
    }
    else {
        write!(c, "Pxe already started\r\n");
    }

    write!(c, "Starting DHCP\r\n");
    match pxe_protocol.dhcp(false) {
        Ok(r) => { write!(c, "Dhcp succeeded\r\n");
                write!(c, "{:?}, {:?}, {:?}\r\n", pxe_protocol.mode().proxy_offer().as_dhcpv4().bootp_opcode(), pxe_protocol.mode().proxy_offer_received(), pxe_protocol.mode().pxe_discover_valid()) },
        Err(e) => write!(c, "Dhcp failed: {:?}\r\n", e),
    };

    let info = DiscoverInfo::default();
    write!(c, "Starting Discover\r\n");
    match pxe_protocol.discover(BootType::Bootstrap, BOOT_LAYER_INITIAL, false, Some(&info)) {
        Ok(_) =>  { 
                    write!(c, "Discover succeeded\r\n");
                    write!(c, "Boot file:\r\n");
                    let boot_file = pxe_protocol.mode().proxy_offer().as_dhcpv4().bootp_boot_file();
                    for ch in boot_file.iter() {
                        write!(c, "{}", *ch as char);
                    }
                    write!(c, "\r\n")
                },

        Err(e) => write!(c, "Discover failed: {:?}\r\n", e),
    };


    write!(c, "Testing TCP4\r\n");
    let remote_ip = net::Ipv4Addr::new(10, 1, 10, 17);
    let remote_port = 1000;

    write!(c, "Connecting to {:?}:{}...\r\n", remote_ip, remote_port);

    net::Tcp4Stream::connect(net::SocketAddrV4::new(remote_ip, remote_port))
        .and_then(|mut stream| {
            write!(c, "Connected!\r\n").unwrap();
            let hello = "Hello";
            let mut bytes = hello.bytes();

            let buf = [bytes.next().unwrap(), bytes.next().unwrap(), bytes.next().unwrap(), bytes.next().unwrap(), bytes.next().unwrap()];

            use io::Write;

            stream.write(&buf);
            write!(c, "Data sent\r\n");
            Ok(())
        })
        .or_else(|e| {
            write!(c, "Got status code: {:?}\r\n", e);
            Err(3)
        });


    
    write!(c, "Testing vec and allocator\r\n");
    let mut v = efi::Vec::new();
    v.push(3);
    v.push(4);
    v.push(8);

    write!(c, "Vec created: {:?} - Capacity: {}\r\n", v, v.capacity());

   0
}