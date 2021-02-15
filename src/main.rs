// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![no_main]
#![feature(try_trait)]

extern crate system76_firmware_smmstore as smmstore;

use core::ops::Try;
use uefi::status::Status;

#[no_mangle]
pub extern "C" fn main() -> Status {
    if let Err(err) = smmstore::smmstore() {
        Status::from_error(err)
    } else {
        Status(0)
    }
}
