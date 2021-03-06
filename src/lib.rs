// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![feature(llvm_asm)]

extern crate rlibc;

use uefi::status::{Error, Result};

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

const CMD_SMMSTORE: u8 = 0xED;

unsafe fn smmstore_cmd(subcmd: u8, arg: u32) -> Result<()> {
    match smm_cmd(CMD_SMMSTORE, subcmd, arg) {
        0 => Ok(()),
        1 => Err(Error::DeviceError),
        2 => Err(Error::Unsupported),
        _ => Err(Error::Unknown),
    }
}

const SMMSTORE_CLEAR: u8 = 1;
const SMMSTORE_READ: u8 = 2;
const SMMSTORE_APPEND: u8 = 3;

pub unsafe fn smmstore_clear() -> Result<()> {
    smmstore_cmd(SMMSTORE_CLEAR, 0)
}

pub unsafe fn smmstore_read(buf: &mut [u8]) -> Result<()> {
    #[repr(C)]
    struct Params {
        buf: u32,
        bufsize: u32,
    }
    let params = Params {
        buf: buf.as_mut_ptr() as u32,
        bufsize: buf.len() as u32
    };
    smmstore_cmd(SMMSTORE_READ, &params as *const Params as u32)
}

pub unsafe fn smmstore_append(key: &[u8], val: &[u8]) -> Result<()> {
    #[repr(C)]
    struct Params {
        key: u32,
        keysize: u32,
        val: u32,
        valsize: u32
    }
    let params = Params {
        key: key.as_ptr() as u32,
        keysize: key.len() as u32,
        val: val.as_ptr() as u32,
        valsize: val.len() as u32
    };
    smmstore_cmd(SMMSTORE_APPEND, &params as *const Params as u32)
}
