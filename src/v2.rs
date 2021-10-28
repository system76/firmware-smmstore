// SPDX-License-Identifier: GPL-3.0-only

// https://doc.coreboot.org/drivers/smmstorev2.html

use uefi::status::Result;
use crate::smmstore_cmd;

/// SMMSTOREv2 blocks must be at least 64 KiB.
/// Read the coreboot table to determine actual block size.
pub const SMM_DEFAULT_BLOCK_SIZE: u32 = 64 * 1024;

// coreboot installs the communication buffer.
//const SMMSTORE_CMD_INIT: u8 = 4;
const SMMSTORE_CMD_RAW_READ: u8 = 5;
const SMMSTORE_CMD_RAW_WRITE: u8 = 6;
const SMMSTORE_CMD_RAW_CLEAR: u8 = 7;

/// Read a block from the SMMSTORE.
pub unsafe fn smmstore_raw_read(block_id: u32, offset: u32, size: u32) -> Result<()> {
    #[repr(C)]
    struct Params {
        size: u32,
        offset: u32,
        block_id: u32,
    }

    let params = Params { size, offset, block_id };
    smmstore_cmd(SMMSTORE_CMD_RAW_READ, &params as *const Params as u32)
}

/// Write a block to the SMMSTORE.
pub unsafe fn smmstore_raw_write(block_id: u32, offset: u32, size: u32) -> Result<()> {
    #[repr(C)]
    struct Params {
        size: u32,
        offset: u32,
        block_id: u32,
    }

    let params = Params { size, offset, block_id };
    smmstore_cmd(SMMSTORE_CMD_RAW_WRITE, &params as *const Params as u32)
}

/// Erase a block from the SMMSTORE.
pub unsafe fn smmstore_raw_clear(block_id: u32) -> Result<()> {
    #[repr(C)]
    struct Params {
        block_id: u32,
    }

    let params = Params {block_id };
    smmstore_cmd(SMMSTORE_CMD_RAW_CLEAR, &params as *const Params as u32)
}
