#![no_std]
#![feature(asm)]
#![feature(prelude_import)]
#![feature(try_trait)]
#![allow(non_snake_case)]

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
        println!("SmmStore error: {:?}", err);
        Status::from_error(err)
    } else {
        Status(0)
    }
}
