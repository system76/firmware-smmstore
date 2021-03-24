use plain::Plain;
use uefi::guid;

pub const FVH_SIGNATURE: &[u8; 4] = b"_FVH";
pub const FVH_REVISION: u8 = 0x02;

// EFI_FIRMWARE_VOLUME_HEADER
#[derive(Clone, Debug)]
#[repr(C)]
pub struct FirmwareVolumeHeader {
    pub zero_vector: [u8; 16],
    pub guid: guid::Guid,
    pub volume_length: u64,
    pub signature: [u8; 4],
    pub attributes: u32,
    pub header_length: u16,
    pub checksum: u16,
    pub ext_header_offset: u16,
    pub reserved: u8,
    pub revision: u8,
    pub block_map: [(u32, u32); 2],
}

unsafe impl Plain for FirmwareVolumeHeader {}

impl FirmwareVolumeHeader {
    pub fn is_valid(&self) -> bool {
        self.zero_vector.iter().all(|&x| x == 0)
            && self.guid == guid::SYSTEM_NV_DATA_FV_GUID
            && self.signature == *FVH_SIGNATURE
            && self.revision == FVH_REVISION
    }
}
