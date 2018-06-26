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
extern crate http_efi;
#[macro_use] extern crate alloc;

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
    io::{self, Read, Write, BufRead, BufReader, Cursor, copy},
    net::TcpStream,
    EfiError,
    EfiErrorKind,
    image,
};

use http_efi::{HttpClient, BufferedStream, Header, Url};
use alloc::{String, string::ToString, Vec};
use core::str;

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

struct HttpReader {
    http_client: HttpClient<BufferedStream>,
    url: Url,
    curr_pos: usize,
}

impl HttpReader {
    // TODO: this should take Into<Url> for ergonomics
    pub fn connect(url: Url) -> efi::Result<Self> {
        let http_client = HttpClient::connect(url.authority())
            .map_err::<EfiError, _>(|_| EfiErrorKind::DeviceError.into())?;

        Ok(Self { http_client, url, curr_pos: 0 })
    }
}

impl Read for HttpReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0)
        }

        let headers = [ Header::new("Range", format!("bytes={}-{}", self.curr_pos, self.curr_pos + buf.len() - 1)) ];
        let mut response = match self.http_client.request("GET", self.url.path(), Some(&headers), None) {
            Ok(res) => Ok(res),
            Err(http_efi::Error::IoError(_)) | Err(http_efi::Error::ParseFailure) => {
                self.http_client = HttpClient::connect(self.url.authority())
                    .map_err::<io::Error, _>(|_| io::ErrorKind::NotConnected.into())?;
                Ok(self.http_client.request("GET", self.url.path(), Some(&headers), None) 
                    .map_err::<io::Error, _>(|_| io::ErrorKind::Other.into())?)
            },
            Err(e) => Err(e),
        }.expect("Response must be valid here");

        if !response.status_code().is_success() {
            return Err(io::ErrorKind::Other.into())
        }

        let mut total_bytes_read = 0;
        loop {
            let bytes_read = response.read(&mut buf[total_bytes_read..])?;
            total_bytes_read += bytes_read;

            if bytes_read == 0 {
                break;
            }
        }

        self.curr_pos += total_bytes_read;

        Ok(total_bytes_read)
    }
}

impl image::Len for HttpReader {
    fn len(&mut self) -> efi::Result<usize> {
        let mut response = self.http_client.request("HEAD", self.url.path(), None, None)
            .map_err::<EfiError, _>(|_| EfiErrorKind::DeviceError.into())?;
        if !response.status_code().is_success() {
            return Err(EfiErrorKind::DeviceError.into())
        }

        let content_len_hdr_buf = &response.headers().iter().find(|h| h.name.eq_ignore_ascii_case("content-length"))
            .ok_or::<EfiError>(EfiErrorKind::DeviceError.into())?
            .value;
        let content_len_val = str::from_utf8(&content_len_hdr_buf)
            .map_err::<EfiError, _>(|_| EfiErrorKind::DeviceError.into())?;
        let len = content_len_val.parse()
            .map_err::<EfiError, _>(|_| EfiErrorKind::DeviceError.into())?;

        Ok(len)
    }
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

    // let info = DiscoverInfo::default();
    // let dp = core::mem::size_of::<DiscoverInfo>();
    // println!("DiscoverInfo size: {}", dp);

    // println!("Starting Discover");
    // match pxe_protocol.discover(BootType::Bootstrap, BOOT_LAYER_INITIAL, false, Some(&info)) {
    //     Ok(_) =>  { 
    //                 println!("Discover succeeded");
    //                 print!("Boot file: ");
    //                 let boot_file = pxe_protocol.mode().unwrap().proxy_offer().unwrap().as_dhcpv4().unwrap().bootp_boot_file();
    //                 for ch in boot_file.iter() {
    //                     print!("{}", *ch as char);
    //                 }
    //                 println!("")
    //             },

    //     Err(e) => return Err(format!("Discover failed - {}", e)),
    // };

    println!("Testing HTTP");

   print!("Enter URL to boot from (or press ENTER for default): ");

    let stdin = efi::stdin();
    let mut url = stdin.lines().next().unwrap().unwrap();
    if url.is_empty() {
        url = "http://client2.ntl.local:8001/wimboot".to_string();
    }

    let url = Url::parse(url).map_err(|_| "bad url")?;
    let mut authority = url.host().to_string();
    if let Some(port) = url.port() {
        authority += ":";
        authority += &port.to_string();
    }

    println!("Booting from '{}'", url);
    let mut reader = HttpReader::connect(url).map_err(|_| "failed to connect HTTP reader")?;
    println!("Connected");
    let loaded_image = image::load_image(&mut reader).map_err(|e| format!("failed to load image: {}", e))?;
    println!("Image loaded");
    let exit_data = image::start_image(&loaded_image).map_err(|_| "failed to start image")?;
    
    // println!("Doing GET on '{}'", url);
    // match HttpClient::connect(authority) {
    //     Ok(mut client) => {
    //         println!("HTTP client connected!");
    //         let mut offset = 0;
    //         let mut req_size = 10;
    //         for _i in 0..1 {
    //             match client.request("GET", url.path(), None, None) {
    //                 Ok(mut resp) => {
    //                     println!("Got status code {} {:?}", resp.status_code().to_u16(), resp.status_code());
    //                     if resp.status_code().is_success() {
    //                         let mut body = String::new();
    //                         resp.read_to_string(&mut body);
    //                         println!("Body:");
    //                         println!("{}", body);
    //                     }
    //                 },
    //                 Err(e) => { return Err(format!("Http client failed to send request - {}", e))},
    //             };

    //             offset += req_size;
    //         }
    //         println!("");
    //     },
    //     Err(e) => return Err(format!("Failed to connect - {}", e))
    // }

    Ok(())
}