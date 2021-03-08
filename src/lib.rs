// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![feature(llvm_asm)]
#![allow(clippy::missing_safety_doc)]

#[macro_use]
extern crate alloc;
extern crate rlibc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::{mem, slice};
use uefi::guid::Guid;
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

/// Check if raw region data is corrupted.
pub fn is_corrupted(data: &[u8]) -> bool {
    let mut i = 0;
    while i + 8 <= data.len() {
        let (keysz, valsz) = unsafe {
            let ptr = data.as_ptr().add(i) as *const u32;
            i += 8;
            (*ptr as usize, *ptr.add(1) as usize)
        };

        // No more entries
        if keysz == 0 || keysz == 0xffff_ffff {
            break;
        }

        // Data too short
        if i + keysz + valsz >= data.len() {
            return true;
        }

        // Check for null byte
        i += keysz + valsz;
        if data[i] != 0 {
            return true;
        }
        i += 1;

        // Align to 32 bits
        i = (i + 3) & !3;
    }

    false
}

/// Determine size in bytes of used data in raw SMMSTORE region.
/// If an entry is corrupted, reports the size up to, but not including, that entry.
pub fn used_size(data: &[u8]) -> usize {
    let mut used = 0;
    let mut i = 0;
    while i + 8 <= data.len() {
        let (keysz, valsz) = unsafe {
            let ptr = data.as_ptr().add(i) as *const u32;
            i += 8;
            (*ptr as usize, *ptr.add(1) as usize)
        };

        // No more entries
        if keysz == 0 || keysz == 0xffff_ffff {
            break;
        }

        // Data too short
        if i + keysz + valsz >= data.len() {
            break;
        }

        // Check for null byte
        i += keysz + valsz;
        if data[i] != 0 {
            break;
        }
        i += 1;

        // Align to 32 bits
        i = (i + 3) & !3;

        // Update used count
        used = i;
    }

    used
}

/// Count the number of duplicate entries in a raw region.
/// Stops if a corrupted entry is encountered.
pub fn count_duplicates(data: &[u8]) -> usize {
    let mut kv = BTreeMap::<&[u8], &[u8]>::new();

    let mut i = 0;
    let mut duplicates = 0;
    while i + 8 <= data.len() {
        let (keysz, valsz) = unsafe {
            let ptr = data.as_ptr().add(i) as *const u32;
            i += 8;
            (*ptr as usize, *ptr.add(1) as usize)
        };

        // No more entries
        if keysz == 0 || keysz == 0xffff_ffff {
            break;
        }

        // Data too short
        if i + keysz + valsz >= data.len() {
            break;
        }

        unsafe {
            let ptr = data.as_ptr().add(i);
            let key = slice::from_raw_parts(
                ptr,
                keysz
            );
            let value = slice::from_raw_parts(
                ptr.add(keysz),
                valsz
            );
            if kv.insert(key, value).is_some() {
                duplicates += 1;
            }
        }

        // Check for null byte
        i += keysz + valsz;
        if data[i] != 0 {
            break;
        }
        i += 1;

        // Align to 32 bits
        i = (i + 3) & !3;
    }

    duplicates
}

/// Convert raw region data into a BTreeMap.
/// Stops if a corrupted entry is encountered.
pub fn deserialize(data: &[u8]) -> BTreeMap::<&[u8], &[u8]> {
    let mut kv = BTreeMap::<&[u8], &[u8]>::new();

    let mut i = 0;
    while i + 8 <= data.len() {
        let (keysz, valsz) = unsafe {
            let ptr = data.as_ptr().add(i) as *const u32;
            i += 8;
            (*ptr as usize, *ptr.add(1) as usize)
        };

        // No more entries
        if keysz == 0 || keysz == 0xffff_ffff {
            break;
        }

        // Data too short
        if i + keysz + valsz >= data.len() {
            break;
        }

        unsafe {
            let ptr = data.as_ptr().add(i);
            let key = slice::from_raw_parts(
                ptr,
                keysz
            );
            let value = slice::from_raw_parts(
                ptr.add(keysz),
                valsz
            );
            kv.insert(key, value);
        }

        // Check for null byte
        i += keysz + valsz;
        if data[i] != 0 {
            break;
        }
        i += 1;

        // Align to 32 bits
        i = (i + 3) & !3;
    }

    kv
}

/// Convert a BTreeMap into raw used data.
pub fn serialize(data: BTreeMap::<&[u8], &[u8]>) -> Vec<u8> {
    let mut raw = Vec::new();

    // Fill in region with new data
    let mut offset = 0;
    for (key, value) in data.iter() {
        if key.len() > mem::size_of::<Guid>() && !value.is_empty() {
            // Key size
            {
                let b: [u8; 4] = unsafe { mem::transmute(key.len() as u32) };
                raw.extend_from_slice(&b);
                offset += mem::size_of::<u32>();
            }

            // Value size
            {
                let b: [u8; 4] = unsafe { mem::transmute(value.len() as u32) };
                raw.extend_from_slice(&b);
                offset += mem::size_of::<u32>();
            }

            // Key
            raw.extend_from_slice(&key);
            offset += key.len();

            // Value
            raw.extend_from_slice(&value);
            offset += value.len();

            // NULL byte
            raw.push(0);
            offset += 1;

            // Align to 32 bits
            while offset % 4 != 0 {
                raw.push(0xFF);
                offset += 1;
            }
        }
    }

    raw
}

/// Convenience function to create new region of same size with compacted data.
pub fn compact(data: &[u8]) -> Vec<u8> {
    let compact = serialize(deserialize(&data));
    let mut new_data = vec![0xFF; data.len()];
    new_data[..compact.len()].copy_from_slice(&compact.as_slice());
    new_data
}
