// SPDX-License-Identifier: GPL-3.0-only

extern crate system76_firmware_smmstore as smmstore;

use std::{char, env, fs, mem, process};
use uefi::guid::Guid;

fn main() {
    let path = match env::args().nth(1) {
        Some(some) => some,
        None => {
            eprintln!("read [file]");
            process::exit(1);
        }
    };

    let data = fs::read(path)
        .expect("failed to read file");

    let compact = smmstore::v1::deserialize(&data);

    for (key, value) in compact.iter() {
        if key.len() > mem::size_of::<Guid>() && !value.is_empty() {
            let (_guid, _varname) = unsafe {
                let ptr = key.as_ptr();
                (
                    *(ptr as *const Guid),
                    ptr.add(mem::size_of::<Guid>()) as *const u16
                )
            };

            print!("\x1B[1m");
            let mut j = mem::size_of::<Guid>();
            while j + 1 < key.len() {
                let w =
                    (key[j] as u16) |
                    (key[j + 1] as u16) << 8;
                if w == 0 {
                    break;
                }
                if let Some(c) = char::from_u32(w as u32) {
                    print!("{}", c);
                }
                j += 2;
            }
            println!(": {}\x1B[0m", value.len());

            for row in 0..(value.len() + 15)/16 {
                print!("{:04X}:", row * 16);
                for col in 0..16 {
                    let j = row * 16 + col;
                    if j < value.len() {
                        print!(" {:02X}", value[j]);
                    }
                }
                println!();
            }
        }
    }

    println!();

    let used = smmstore::v1::used_size(&data);
    let percent = (used * 100) / data.len();
    println!("\x1B[1mSMMSTORE used space:\x1B[0m {} / {} bytes ({}%)", used, data.len(), percent);

    if smmstore::v1::is_corrupted(&data) {
        println!("\x1B[1m\x1B[91mSMMSTORE region is corrupted\x1B[39m\x1B[0m");
    }
}
