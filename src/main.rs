// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![no_main]
#![feature(try_trait)]

extern crate alloc;
extern crate system76_firmware_smmstore as smmstore;
extern crate uefi_std as std;

use alloc::collections::BTreeMap;
use core::ops::Try;
use std::{mem, slice};
use uefi::guid::Guid;
use uefi::status::{Result, Status};

fn smmstore() -> Result<()> {
    let mut data = [0; 0x40000];
    let res = unsafe { smmstore::smmstore_read(&mut data) };
    // println!("Read {:?}", res);
    res?;

    let mut compact = BTreeMap::<&[u8], &[u8]>::new();

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
            if compact.insert(key, value).is_some() {
                duplicates += 1;
            }
        }

        i += keysz + valsz + 1;
        i = (i + 3) & !3;
    }

    let rewrite = {
        i >= data.len() / 2 ||
        duplicates >= 16
    };

    if rewrite {
        let res = unsafe { smmstore::smmstore_clear() };
        // println!("Clear {:?}", res);
        res?;

        for (key, value) in compact.iter() {
            if key.len() > mem::size_of::<Guid>() && value.len() > 0 {
                let res = unsafe { smmstore::smmstore_append(&key, &value) };
                // println!("Append {:?}", res);
                res?;
            }
        }
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn main() -> Status {
    if let Err(err) = smmstore() {
        Status::from_error(err)
    } else {
        Status(0)
    }
}
