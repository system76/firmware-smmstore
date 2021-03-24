use coreboot_table::{Mapper, PhysicalAddress, SmmstoreV2, Table, VirtualAddress};

pub struct PhysicalMapper;

impl Mapper for PhysicalMapper {
    unsafe fn map_aligned(&mut self, address: PhysicalAddress, _size: usize) -> Result<VirtualAddress, &'static str> {
        Ok(VirtualAddress(address.0))
    }

    unsafe fn unmap_aligned(&mut self, _address: VirtualAddress) -> Result<(), &'static str> {
        Ok(())
    }

    fn page_size(&self) -> usize {
        4096
    }
}

pub fn smmstorev2_record() -> Option<SmmstoreV2> {
    let mut tag = None;

    let _ = coreboot_table::tables(|table| {
        match table {
            Table::SmmstoreV2(smmstore) => {
                // Now what?
                tag = Some(smmstore.clone());
            },
            _ => (),
        }
        Ok(())
    }, &mut PhysicalMapper);

    tag
}

