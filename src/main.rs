// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![no_main]
#![feature(try_trait_v2)]

extern crate alloc;
extern crate system76_firmware_smmstore as smmstore;
extern crate uefi_std as std;

use core::ops::FromResidual;
use core::mem;
use std::uefi::guid::Guid;
use std::uefi::status::{Result, Status};

fn smmstore() -> Result<()> {
    let mut data = [0; 0x40000];
    let res = unsafe { smmstore::smmstore_read(&mut data) };
    res?;

    let rewrite = {
        smmstore::used_size(&data) >= data.len() / 2 ||
        smmstore::count_duplicates(&data) >= 16
    };

    if rewrite {
        let res = unsafe { smmstore::smmstore_clear() };
        res?;

        let compact = smmstore::deserialize(&data);
        for (key, value) in compact.iter() {
            if key.len() > mem::size_of::<Guid>() && !value.is_empty() {
                let res = unsafe { smmstore::smmstore_append(key, value) };
                res?;
            }
        }
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn main() -> Status {
    if let Err(err) = smmstore() {
        Status::from_residual(err)
    } else {
        Status(0)
    }
}
