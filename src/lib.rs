// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![feature(llvm_asm)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::manual_flatten)]

#[macro_use]
extern crate alloc;

use uefi::status::{Error, Result};

pub mod v1;

const CMD_SMMSTORE: u8 = 0xED;

/// Trigger an SMI by writing to APM_CNT.
unsafe fn smm_cmd(cmd: u8, subcmd: u8, arg: u32) -> u32 {
    let res;
    llvm_asm!(
        "out 0xB2, $0"
        : "={eax}"(res)
        : "{eax}"(((subcmd as u32) << 8) | (cmd as u32)), "{ebx}"(arg)
        : "memory"
        : "intel", "volatile"
    );
    res
}

unsafe fn smmstore_cmd(subcmd: u8, arg: u32) -> Result<()> {
    match smm_cmd(CMD_SMMSTORE, subcmd, arg) {
        0 => Ok(()),
        1 => Err(Error::DeviceError),
        2 => Err(Error::Unsupported),
        _ => Err(Error::Unknown),
    }
}
