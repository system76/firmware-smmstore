// SPDX-License-Identifier: GPL-3.0-only

// https://doc.coreboot.org/drivers/smmstore.html

use alloc::{collections::BTreeMap, vec::Vec};
use core::mem;
use uefi::{guid::Guid, status::{Error, Result}};
use crate::smmstore_cmd;

const SMMSTORE_CLEAR: u8 = 1;
const SMMSTORE_READ: u8 = 2;
const SMMSTORE_APPEND: u8 = 3;

/// Clear the entire SMMSTORE region.
pub unsafe fn smmstore_clear() -> Result<()> {
    smmstore_cmd(SMMSTORE_CLEAR, 0)
}

/// Read the SMMSTORE region.
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

/// Append an entry to the SMMSTORE data.
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

/// Simple key-value pair representing an SMMSTORE entry.
struct Entry {
    key: Vec<u8>,
    value: Vec<u8>,
}

struct Smmstore {
    data: Vec<u8>,
    offset: usize,
}

impl Smmstore {
    pub fn from_raw(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            offset: 0,
        }
    }
}

impl Iterator for Smmstore {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 8 <= self.data.len() {
            let (keysz, valsz) = unsafe {
                let ptr = self.data.as_ptr().add(self.offset) as *const u32;
                self.offset += 8;
                (*ptr as usize, *ptr.add(1) as usize)
            };

            // No more entries
            if keysz == 0 || keysz == 0xffff_ffff {
                return None;
            }

            // Data too short
            if self.offset + keysz + valsz >= self.data.len() {
                return Some(Err(Error::DeviceError));
            }

            let key = self.data[self.offset..self.offset + keysz].to_vec();
            self.offset += keysz;

            let value = self.data[self.offset..self.offset + valsz].to_vec();
            self.offset += valsz;

            // Check for null byte
            if self.data[self.offset] != 0 {
                return Some(Err(Error::DeviceError));
            }
            self.offset += 1;

            // Align to 4 bytes
            self.offset = (self.offset + 3) & !3;

            Some(Ok(Entry { key, value }))
        } else {
            None
        }
    }
}


/// Check if SMMSTORE data is corrupted.
pub fn is_corrupted(data: &[u8]) -> bool {
    let smmstore = Smmstore::from_raw(data);
    for entry in smmstore {
        if let Err(_err) = entry {
            return true;
        }
    }

    false
}

/// Determine size in bytes of used data in the SMMSTORE data.
///
/// No error is returned if a corrupted entry is encountered. Instead, the used
/// size up to the corrupted entry is returned.
pub fn used_size(data: &[u8]) -> usize {
    let smmstore = Smmstore::from_raw(data);

    let mut used = 0;
    for entry in smmstore {
        if let Ok(entry) = entry {
            // Key size + value size + key + value + NULL byte + alignment
            used += (8 + entry.key.len() + entry.value.len() + 1 + 3) & !3;
        }
    }

    used
}

/// Count the number of duplicate entries in a raw region.
///
/// No error is returned if a corrupted entry is encountered. Instead, the
/// number of duplicates up to that point is returned.
pub fn count_duplicates(data: &[u8]) -> usize {
    let mut kv = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    let mut duplicates = 0;

    let smmstore = Smmstore::from_raw(data);
    for entry in smmstore {
        if let Ok(entry) = entry {
            if kv.insert(entry.key, entry.value).is_some() {
                duplicates += 1;
            }
        }
    }

    duplicates
}

/// Convert raw region data into a BTreeMap.
///
/// No error is returned if a corrupted entry is encountered. Instead, a
/// BTreeMap of all values up to that point is returned.
pub fn deserialize(data: &[u8]) -> BTreeMap::<Vec<u8>, Vec<u8>> {
    let mut kv = BTreeMap::<Vec<u8>, Vec<u8>>::new();

    let smmstore = Smmstore::from_raw(data);
    for entry in smmstore {
        if let Ok(entry) = entry {
            kv.insert(entry.key, entry.value);
        }
    }

    kv
}

/// Convert a BTreeMap into raw used data.
pub fn serialize(data: BTreeMap::<Vec<u8>, Vec<u8>>) -> Vec<u8> {
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
            raw.extend_from_slice(key);
            offset += key.len();

            // Value
            raw.extend_from_slice(value);
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
    let compact = serialize(deserialize(data));
    let mut new_data = vec![0xFF; data.len()];
    new_data[..compact.len()].copy_from_slice(compact.as_slice());
    new_data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_used_sized() {
        let data: &[u8] = &[
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x37, 0x00, 0x36, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
            // Filler
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        assert!(!is_corrupted(data));
        assert_eq!(used_size(data), 40);
    }

    #[test]
    fn test_odd_sized() {
        let data: &[u8] = &[
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x01, 0x00, 0x36,
            // 0 byte and padding
            0x00,
            // Filler
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        assert!(!is_corrupted(data));
        assert_eq!(used_size(data), 36);
    }

    #[test]
    fn test_value_size_too_large() {
        let data: &[u8] = &[
            // Key size
            0x18, 0x00, 0x00, 0x00,
            // Value size that is too large
            0x10, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x37, 0x00, 0x36, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
            // Filler
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        assert!(is_corrupted(data));
        assert_eq!(used_size(data), 0);
    }

    #[test]
    fn test_duplicate_entry() {
        let data: &[u8] = &[
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x01, 0x00, 0x00, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x02, 0x00, 0x00, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
        ];

        assert_eq!(count_duplicates(data), 1);
    }


    #[test]
    fn test_compact_without_duplicate() {
        let data: &[u8] = &[
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x37, 0x00, 0x36, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
            // Filler
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        assert_eq!(compact(data), data);
    }

    #[test]
    fn test_compact_with_duplicate() {
        let data: &[u8] = &[
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x37, 0x00, 0x36, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
            // Key and value size
            0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            // Key
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            // Value
            0x37, 0x00, 0xFF, 0x00,
            // 0 byte and padding
            0x00, 0xFF, 0xFF, 0xFF,
        ];

        let compacted = compact(data);
        assert_ne!(compacted, data);
        assert_eq!(count_duplicates(&compacted), 0);
    }
}

