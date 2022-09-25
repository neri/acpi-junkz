#![feature(abi_efiapi)]
#![no_std]
#![no_main]
use core::{ffi::c_void, fmt::Write, mem::MaybeUninit, panic::PanicInfo};
use myacpi::*;
use uefi::{prelude::*, table::cfg::ACPI2_GUID};

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

#[entry]
fn efi_main(_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    unsafe {
        ST.write(&mut st);
    }

    let rsdptr = st.find_config_table(ACPI2_GUID).unwrap();
    let acpi = unsafe { RsdPtr::parse(rsdptr) }.unwrap();
    let xsdt = acpi.xsdt();

    writeln!(
        st.stdout(),
        "XSDT {:08x} {} {}",
        xsdt as *const _ as usize,
        xsdt.header().len(),
        xsdt.table_count(),
    )
    .unwrap();

    for table in xsdt.tables() {
        writeln!(
            st.stdout(),
            "{} {:08x} {}",
            table.signature(),
            table as *const _ as usize,
            table.len(),
        )
        .unwrap();
    }

    Status::SUCCESS
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
