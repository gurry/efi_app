#![no_std]
#![feature(intrinsics)]
#![feature(asm)]
#![feature(lang_items)]
#![feature(link_args)]
#![feature(compiler_builtins_lib)]
#![feature(alloc)]

#[allow(unused_attributes)] // The below attribute is needed to specify the entry point. Hence suppressing the warning
#[link_args = "/ENTRY:efi_start"]
extern "C" {}

extern crate rlibc;
extern crate compiler_builtins;
#[macro_use] extern crate efi;
extern crate http_efi;
extern crate alloc;

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
    io::{self, Read, Write, BufRead, BufReader},
    net::TcpStream,
};

use http_efi::{Client, Header, Url};
use alloc::{String, string::ToString};

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

fn run(sys_table: &mut SystemTable) -> Result<(), String> {
    let mut c = efi::stdout();

    println!("Hello from UEFI");

    let mut bs = sys_table.boot_services();

    println!("Testing DHCP/PXE");
    // TODO: see tianocore-edk2\NetworkPkg\UefiPxeBcDxe\PxeBcBoot.c file to know to implement PXE sequence especially the method PxeBcDiscoverBootFile
    let pxe_protocol = bs.locate_protocol::<PxeBaseCodeProtocol>().map_err(|e| "failed to locate Pxe prtocol")?;

    let sp = core::mem::size_of::<PxeBaseCodeProtocol>();
    println!("PXE size: {}", sp);

    if !pxe_protocol.mode().unwrap().started() {
        println!("Starting Pxe");
        pxe_protocol.start(false);
    }
    else {
        println!("Pxe already started");
    }

    println!("Starting DHCP");
    match pxe_protocol.dhcp(false) {
        Ok(r) => { println!("Dhcp succeeded") },
                // println!("{:?}, {:?}, {:?}", pxe_protocol.mode().proxy_offer().as_dhcpv4().bootp_opcode(), pxe_protocol.mode().proxy_offer_received(), pxe_protocol.mode().pxe_discover_valid()) },
        Err(e) => return Err(format!("Dhcp failed - {}", e))
    };

    let info = DiscoverInfo::default();
    let dp = core::mem::size_of::<DiscoverInfo>();
    println!("DiscoverInfo size: {}", dp);

    println!("Starting Discover");
    match pxe_protocol.discover(BootType::Bootstrap, BOOT_LAYER_INITIAL, false, Some(&info)) {
        Ok(_) =>  { 
                    println!("Discover succeeded");
                    print!("Boot file: ");
                    let boot_file = pxe_protocol.mode().unwrap().proxy_offer().unwrap().as_dhcpv4().unwrap().bootp_boot_file();
                    for ch in boot_file.iter() {
                        print!("{}", *ch as char);
                    }
                    println!("")
                },

        Err(e) => return Err(format!("Discover failed - {}", e)),
    };


    println!("Testing HTTP");

    print!("Enter HTTP URL: ");

    let stdin = efi::stdin();
    let url = stdin.lines().next().unwrap().unwrap();
    let url = Url::parse(url).map_err(|_| "bad url")?;

    let mut authority = url.host().to_string();
    if let Some(port) = url.port() {
        authority += ":";
        authority += &port.to_string();
    }
    
    match Client::connect(authority) {
        Ok(mut client) => {
            println!("HTTP client connected!");
            let mut offset = 0;
            let mut req_size = 10;
            for _i in 0..1 {
                // let mut range_val = format!("bytes={}-{}", offset, offset + req_size);
                // let headers = [ Header::new("Range", range_val) ];
                match client.request("GET", url.path(), None, None) {
                    Ok(mut resp) => {
                        println!("Got status code {} {:?}", resp.status_code().to_u16(), resp.status_code());
                        if resp.status_code().is_success() {
                            let mut body = String::new();
                            resp.read_to_string(&mut body);
                            println!("Body:");
                            println!("{}", body);
                        }
                    },
                    Err(e) => { return Err(format!("Http client failed to connect - {}", e))},
                };

                offset += req_size;
            }
            println!("");
        },
        Err(e) => println!("Got status code: {:?}", e)
    }

    Ok(())
}