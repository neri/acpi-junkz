#![feature(abi_efiapi)]
#![no_std]
#![no_main]
use core::{ffi::c_void, fmt::Write, mem::MaybeUninit, panic::PanicInfo};
use myacpi::{madt::*, *};
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
    let madt = match acpi.xsdt().find_first::<Madt>() {
        Some(v) => v,
        None => {
            writeln!(st.stderr(), "MADT not found").unwrap();
            return Status::LOAD_ERROR;
        }
    };

    writeln!(
        st.stdout(),
        "LocalApic {:08x} has_8259 {}",
        madt.local_apic_address(),
        madt.has_8259(),
    )
    .unwrap();

    for entry in madt.all_entries() {
        match entry {
            MadtEntry::LocalApic(lapic) => {
                writeln!(
                    st.stdout(),
                    "LocalApic UID {} APIC_ID {} status {:?}",
                    lapic.uid(),
                    lapic.apic_id(),
                    lapic.status(),
                )
                .unwrap();
            }
            MadtEntry::IoApic(ioapic) => {
                writeln!(
                    st.stdout(),
                    "IoApic APIC_ID {} GSI_BASE {} BASE {:08x}",
                    ioapic.apic_id(),
                    ioapic.gsi_base(),
                    ioapic.io_apic_address(),
                )
                .unwrap();
            }
            MadtEntry::InterruptSourceOverride(iso) => {
                writeln!(
                    st.stdout(),
                    "InterruptSourceOverride BUS {} SOURCE {} GSI {} flags {:04x}",
                    iso.bus(),
                    iso.source(),
                    iso.global_system_interrupt(),
                    iso.flags(),
                )
                .unwrap();
            }
            MadtEntry::Other(entry) => {
                writeln!(st.stdout(), "???: {:?} {}", entry.entry_type(), entry.len()).unwrap();
            }
            _ => unimplemented!(),
        }
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
