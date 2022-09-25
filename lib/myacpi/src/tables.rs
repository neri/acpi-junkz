use core::{ffi::c_void, fmt::Display, mem::transmute, str::from_utf8_unchecked};

/// Root System Description Pointer
#[repr(C, packed)]
#[allow(unused)]
pub struct RsdPtr {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    rev: u8,
    rsdt_addr: u32,
    len: u32,
    xsdt_addr: u64,
    checksum2: u8,
    _reserved: [u8; 3],
}

impl RsdPtr {
    pub const VALID_SIGNATURE: [u8; 8] = *b"RSD PTR ";
    pub const CURRENT_REV: u8 = 2;

    pub unsafe fn parse(ptr: *const c_void) -> Option<&'static Self> {
        let p = ptr as *const Self;
        let p = unsafe { &*p };
        p.is_valid().then(|| p)
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.signature == Self::VALID_SIGNATURE && self.rev == Self::CURRENT_REV
    }

    #[inline]
    pub fn xsdt(&self) -> &Xsdt {
        unsafe { &*(self.xsdt_addr as usize as *const Xsdt) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableId(pub [u8; 4]);

impl TableId {
    /// Extended System Description Table
    pub const XSDT: Self = Self(*b"XSDT");

    /// Fixed ACPI Description Table
    pub const FADT: Self = Self(*b"FACP");

    /// Multiple APIC Description Table
    pub const MADT: Self = Self(*b"APIC");

    /// High Precision Event Timers
    pub const HPET: Self = Self(*b"HPET");

    /// Boot Graphics Resource Table
    pub const BGRT: Self = Self(*b"BGRT");
}

impl TableId {
    pub const fn as_str(&self) -> &str {
        unsafe { from_utf8_unchecked(&self.0) }
    }
}

impl Display for TableId {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[repr(C)]
#[allow(unused)]
pub struct AcpiHeader {
    signature: TableId,
    len: u32,
    rev: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_rev: u32,
    creator_id: u32,
    creator_rev: u32,
}

impl AcpiHeader {
    #[inline]
    pub const fn signature(&self) -> TableId {
        self.signature
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    fn identify<T: AcpiTable>(&self) -> Option<&T> {
        (self.signature() == T::TABLE_ID).then(|| unsafe { transmute(self) })
    }
}

pub unsafe trait AcpiTable {
    const TABLE_ID: TableId;

    fn header(&self) -> &AcpiHeader;
}

/// Generic Address Structure (GAS)
#[repr(C, packed)]
#[allow(unused)]
pub struct Gas {
    id: GasAddressSpaceId,
    bit_width: u8,
    bit_offset: u8,
    access_size: GasAccessSize,
    address: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum GasAddressSpaceId {
    /// System Memory space
    SystemMemory = 0,
    /// System I/O space
    SystemIo,
    /// PCI Configuration space
    PciConfiguration,
    /// Embedded Controller
    EmbeddedController,
    /// SMBus
    SmBus,
    /// SystemCMOS
    SystemCmos,
    /// PciBarTarget
    PciBarTarget,
    /// IPMI
    Ipmi,
    /// General PurposeIO
    Gpio,
    /// GenericSerialBus
    GenericSerialBus,
    /// Platform Communications Channel (PCC)
    Pcc,
    /// Platform Runtime Mechanism (PRM)
    Prm,
    /// Functional Fixed Hardware
    FunctionalFixedHardware = 0x7F,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum GasAccessSize {
    Undefined = 0,
    Byte,
    Word,
    Dword,
    Qword,
}

/// Extended System Description Table
#[repr(C, packed)]
pub struct Xsdt {
    _hdr: AcpiHeader,
    _entry: u64,
}

unsafe impl AcpiTable for Xsdt {
    const TABLE_ID: TableId = TableId::XSDT;

    #[inline]
    fn header(&self) -> &AcpiHeader {
        unsafe { transmute(self) }
    }
}

impl Xsdt {
    #[inline]
    pub fn tables<'a>(&'a self) -> impl Iterator<Item = &'a AcpiHeader> {
        XsdtWalker {
            xsdt: self,
            index: 0,
        }
    }

    #[inline]
    pub fn table_count(&self) -> usize {
        (self.header().len() - 32) / 8
    }

    pub fn find_first<T: AcpiTable>(&self) -> Option<&T> {
        self.tables()
            .find(|v| v.signature() == T::TABLE_ID)
            .and_then(|v| v.identify())
    }
}

struct XsdtWalker<'a> {
    xsdt: &'a Xsdt,
    index: usize,
}

impl<'a> Iterator for XsdtWalker<'a> {
    type Item = &'a AcpiHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.index * 8 + 36;
        if offset >= self.xsdt.header().len() {
            return None;
        } else {
            self.index += 1;

            Some(unsafe {
                &*(((self.xsdt as *const _ as *const c_void).add(offset) as *const u64)
                    .read_unaligned() as usize as *const AcpiHeader)
            })
        }
    }
}

/// Boot Graphics Resource Table
#[repr(C, packed)]
pub struct Bgrt {
    _hdr: AcpiHeader,
    version: u16,
    status: u8,
    image_type: u8,
    image_address: u64,
    offset_x: u32,
    offset_y: u32,
}

unsafe impl AcpiTable for Bgrt {
    const TABLE_ID: TableId = TableId::BGRT;

    #[inline]
    fn header(&self) -> &AcpiHeader {
        unsafe { transmute(self) }
    }
}

impl Bgrt {
    #[inline]
    pub fn bitmap(&self) -> *const u8 {
        self.image_address as usize as *const u8
    }

    #[inline]
    pub const fn offset(&self) -> (usize, usize) {
        (self.offset_x as usize, self.offset_y as usize)
    }
}
