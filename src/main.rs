// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![no_main]
#![feature(try_trait_v2)]

extern crate alloc;
extern crate system76_firmware_smmstore as smmstore;
extern crate uefi_std as std;

use core::ops::FromResidual;
use core::mem;
use uefi::guid::Guid;
use uefi::status::{Result, Status};

fn smmstore() -> Result<()> {
    let mut data = [0; 0x40000];
    unsafe { smmstore::v1::smmstore_read(&mut data)? };

    let rewrite = {
        smmstore::v1::used_size(&data) >= data.len() / 2 ||
        smmstore::v1::count_duplicates(&data) >= 16
    };

    if rewrite {
        unsafe { smmstore::v1::smmstore_clear()? };

        let compact = smmstore::v1::deserialize(&data);
        for (key, value) in compact.iter() {
            if key.len() > mem::size_of::<Guid>() && !value.is_empty() {
                unsafe { smmstore::v1::smmstore_append(key, value)? };
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
