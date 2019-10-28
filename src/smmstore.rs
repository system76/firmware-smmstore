use std::{mem, slice};
use std::collections::BTreeMap;
use uefi::guid::Guid;
use uefi::status::{Error, Result};

unsafe fn smm_cmd(cmd: u8, subcmd: u8, arg: u32) -> u32 {
    let res;
    asm!(
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

unsafe fn smmstore_clear() -> Result<()> {
    smmstore_cmd(SMMSTORE_CLEAR, 0)
}

unsafe fn smmstore_read(buf: &mut [u8]) -> Result<()> {
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

unsafe fn smmstore_append(key: &[u8], val: &[u8]) -> Result<()> {
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

pub fn smmstore() -> Result<()> {
    let mut data = [0; 0x40000];
    let res = unsafe { smmstore_read(&mut data) };
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
        duplicates >= 4
    };

    if rewrite {
        let res = unsafe { smmstore_clear() };
        // println!("Clear {:?}", res);
        res?;
    
        for (key, value) in compact.iter() {
            if key.len() > mem::size_of::<Guid>() && value.len() > 0 {
                let res = unsafe { smmstore_append(&key, &value) };
                // println!("Append {:?}", res);
                res?;
            }
        }
    }

    Ok(())
}
