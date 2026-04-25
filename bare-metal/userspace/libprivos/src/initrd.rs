// Initrd reader for Privion OS.
//
// The initrd is a flat binary archive embedded in the init binary at
// compile time via include_bytes!. It contains all service binaries
// (cryptod, vfs, netd, driverd, compositor) packed by build-initrd.sh.
//
// Format (little-endian, no padding between entries):
//
//   [4 bytes]  magic: b"PRIV"
//   [4 bytes]  entry count (u32)
//   For each entry:
//     [32 bytes] name (null-terminated, remaining bytes zero)
//     [8 bytes]  offset of ELF data from start of archive (u64)
//     [8 bytes]  size of ELF data in bytes (u64)
//   [ELF data...] (concatenated, at the offsets stated above)

use core::str;

pub const MAGIC: &[u8; 4] = b"PRIV";
const ENTRY_HEADER_SIZE: usize = 32 + 8 + 8; // name + offset + size

#[derive(Debug)]
pub enum InitrdError {
    BadMagic,
    Truncated,
    NotFound,
    InvalidUtf8,
}

/// A parsed view of the embedded initrd archive.
pub struct Initrd<'a> {
    data: &'a [u8],
    count: u32,
}

impl<'a> Initrd<'a> {
    /// Parse the initrd from a byte slice (e.g. from include_bytes!).
    pub fn parse(data: &'a [u8]) -> Result<Self, InitrdError> {
        if data.len() < 8 { return Err(InitrdError::Truncated); }
        if &data[0..4] != MAGIC { return Err(InitrdError::BadMagic); }
        let count = u32::from_le_bytes(data[4..8].try_into().unwrap());
        Ok(Self { data, count })
    }

    /// Find a service binary by name and return its ELF bytes.
    pub fn find(&self, name: &str) -> Result<&'a [u8], InitrdError> {
        let mut cursor = 8usize;
        for _ in 0..self.count {
            if cursor + ENTRY_HEADER_SIZE > self.data.len() {
                return Err(InitrdError::Truncated);
            }

            // Read the name (up to first null byte)
            let name_bytes = &self.data[cursor..cursor + 32];
            let nul = name_bytes.iter().position(|&b| b == 0).unwrap_or(32);
            let entry_name = str::from_utf8(&name_bytes[..nul])
                .map_err(|_| InitrdError::InvalidUtf8)?;

            let offset = u64::from_le_bytes(
                self.data[cursor + 32..cursor + 40].try_into().unwrap()
            ) as usize;
            let size = u64::from_le_bytes(
                self.data[cursor + 40..cursor + 48].try_into().unwrap()
            ) as usize;

            if entry_name == name {
                let end = offset.checked_add(size).ok_or(InitrdError::Truncated)?;
                if end > self.data.len() {
                    return Err(InitrdError::Truncated);
                }
                return Ok(&self.data[offset..end]);
            }

            cursor += ENTRY_HEADER_SIZE;
        }
        Err(InitrdError::NotFound)
    }
}
