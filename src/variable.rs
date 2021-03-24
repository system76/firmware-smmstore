use plain::Plain;
use uefi::guid;

// VARIABLE_STORE_HEADER
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct VariableStoreHeader {
    pub signature: guid::Guid,
    pub size: u32,
    pub format: u8,
    pub state: u8,
    pub reserved: u16,
    pub reserved1: u32,
}

unsafe impl Plain for VariableStoreHeader {}

impl VariableStoreHeader {
    pub fn is_valid(&self) -> bool {
        self.signature == guid::AUTHENTICATED_VARIABLE_GUID
    }
}

pub const VARIABLE_START_ID: u16 = 0x55AA;

// AUTHENTICATED_VARIABLE_HEADER
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct AuthenticatedVariableHeader {
    pub start_id: u16,
    pub state: u8,
    pub reserved: u8,
    pub attributes: u32,
    pub monotonic_count: u64,
    pub timestamp: [u8; 16],
    pub pubkey_index: u32,
    pub name_size: u32,
    pub data_size: u32,
    pub vendor_guid: guid::Guid,
}

unsafe impl Plain for AuthenticatedVariableHeader {}

