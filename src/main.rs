// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![no_main]
#![feature(llvm_asm)]
#![feature(prelude_import)]
#![feature(try_trait)]
#![allow(non_snake_case)]

extern crate rlibc;
#[macro_use]
extern crate uefi_std as std;

#[allow(unused_imports)]
#[prelude_import]
use std::prelude::*;

use core::ops::Try;
use uefi::status::Status;

mod smmstore;

#[no_mangle]
pub extern "C" fn main() -> Status {
    if let Err(err) = smmstore::smmstore() {
        Status::from_error(err)
    } else {
        Status(0)
    }
}
