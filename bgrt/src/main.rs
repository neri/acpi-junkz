#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]

extern crate alloc;
use alloc::{slice, vec::Vec};
use core::{alloc::Layout, ffi::c_void, fmt::Write, mem::MaybeUninit, panic::PanicInfo};
use myacpi::{bgrt::Bgrt, *};
use uefi::{
    prelude::*,
    proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput},
    table::cfg::ACPI2_GUID,
};

static mut ST: MaybeUninit<*mut SystemTable<Boot>> = MaybeUninit::uninit();

#[inline]
fn system_table() -> &'static mut SystemTable<Boot> {
    unsafe { &mut *ST.assume_init() }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    writeln!(system_table().stderr(), "{}", info).unwrap();
    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[entry]
fn efi_main(_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    unsafe {
        ST.write(&mut st);
        uefi::alloc::init(st.boot_services());
    }

    let rsdptr = st.find_config_table(ACPI2_GUID).unwrap();
    let acpi = unsafe { RsdPtr::parse(rsdptr) }.unwrap();
    let xsdt = acpi.xsdt();

    let bgrt = match xsdt.find_first::<Bgrt>() {
        Some(v) => v,
        None => {
            writeln!(st.stderr(), "BGRT Table not found").unwrap();
            return Status::LOAD_ERROR;
        }
    };

    let gop = match unsafe { st.boot_services().locate_protocol::<GraphicsOutput>() } {
        Ok(v) => unsafe { &mut *v.get() },
        Err(_) => {
            writeln!(st.stderr(), "GOP not found").unwrap();
            return Status::LOAD_ERROR;
        }
    };

    let (w, h, buffer) = unsafe { convert_bmp(bgrt.bitmap()) };

    gop.blt(BltOp::BufferToVideo {
        buffer: buffer.as_slice(),
        src: BltRegion::Full,
        dest: bgrt.offset(),
        dims: (w, h),
    })
    .unwrap();

    Status::SUCCESS
}

unsafe fn convert_bmp(ptr: *const u8) -> (usize, usize, Vec<BltPixel>) {
    let bmp_w = unsafe { (ptr.add(18) as *const u32).read_unaligned().to_le() as usize };
    let bmp_h = unsafe { (ptr.add(22) as *const u32).read_unaligned().to_le() as usize };
    let bmp_bpp = unsafe { (ptr.add(28) as *const u16).read_unaligned().to_le() as usize };
    let bmp_bpp8 = (bmp_bpp + 7) / 8;
    let bmp_delta = (bmp_bpp8 * bmp_w + 3) & !3;
    let dib = slice::from_raw_parts(
        unsafe { ptr.add((ptr.add(10) as *const u32).read_unaligned().to_le() as usize) },
        bmp_delta * bmp_h,
    );

    let mut vec = Vec::with_capacity(bmp_w * bmp_h);

    match bmp_bpp {
        24 | 32 => {
            for i in (0..bmp_h).rev() {
                let slice = &dib[i * bmp_delta..];
                for j in 0..bmp_w {
                    vec.push(BltPixel::new(
                        slice[j * bmp_bpp8 + 2],
                        slice[j * bmp_bpp8 + 1],
                        slice[j * bmp_bpp8],
                    ));
                }
            }
        }
        _ => (),
    }

    (bmp_w, bmp_h, vec)
}

pub trait MyUefiLib {
    fn find_config_table(&self, _: ::uefi::Guid) -> Option<*const c_void>;
}

impl MyUefiLib for SystemTable<::uefi::table::Boot> {
    fn find_config_table(&self, guid: ::uefi::Guid) -> Option<*const c_void> {
        for entry in self.config_table() {
            if entry.guid == guid {
                return Some(entry.address);
            }
        }
        None
    }
}
