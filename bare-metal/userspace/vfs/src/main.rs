#![no_std]
#![no_main]

// Encrypted virtual filesystem server for Privion OS.
//
// Phase 6: in-memory file store with ChaCha20-Poly1305 authenticated encryption.
//
// CSpace layout (caps granted by init):
//   slot 0: vfs_ep — endpoint for receiving file operation requests
//
// IPC protocol (seL4 fast-path ≤ 4 message registers, 32 bytes max):
//
//   WRITE: info(label=0x0101, len=3)
//     MR0 = file_key (u64 identifier — callers hash their filename to this)
//     MR1 = data_lo  (first 8 bytes of plaintext as little-endian u64)
//     MR2 = data_hi  (next  8 bytes of plaintext as little-endian u64)
//   Reply OK:  info(label=0x0000, len=0)
//   Reply ERR: info(label=0x0001, len=1), MR0 = error code
//
//   READ: info(label=0x0100, len=1)
//     MR0 = file_key
//   Reply OK:  info(label=0x0000, len=2), MR0=data_lo, MR1=data_hi
//   Reply ERR: info(label=0x0001, len=1), MR0 = error code
//
// Storage: up to 16 files, 16 bytes of plaintext each.
// Master key: hardcoded in Phase 6; Phase 7 derives it from cryptod via Argon2.

use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit};
use chacha20poly1305::aead::AeadInPlace;
use sel4_sys::native;

// ---------------------------------------------------------------------------
// IPC label constants (mirrors libprivos::ipc::labels)
// ---------------------------------------------------------------------------

const MSG_OK:          u64 = 0x0000;
const MSG_ERROR:       u64 = 0x0001;
const MSG_READ_BLOCK:  u64 = 0x0100;
const MSG_WRITE_BLOCK: u64 = 0x0101;

// VFS error codes (carried in MR0 of an MSG_ERROR reply).
const ERR_NOT_FOUND: u64 = 1;
const ERR_NO_SPACE:  u64 = 2;
const ERR_CRYPTO:    u64 = 3;

// CSpace slot layout.
const SLOT_VFS_EP: u64 = 0;

// ---------------------------------------------------------------------------
// Encryption parameters
// ---------------------------------------------------------------------------

// Phase 6: hardcoded master key.
// Phase 7: replaced with a key derived from cryptod using Argon2id.
const MASTER_KEY: [u8; 32] = [
    0x50, 0x72, 0x69, 0x76, 0x69, 0x6f, 0x6e, 0x56,
    0x46, 0x53, 0x4b, 0x65, 0x79, 0x50, 0x68, 0x61,
    0x73, 0x65, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Derive a 12-byte nonce from the file key.
/// Uses the key as the first 8 bytes; remaining 4 are zero.
#[inline]
fn file_nonce(key: u64) -> [u8; 12] {
    let mut n = [0u8; 12];
    n[..8].copy_from_slice(&key.to_le_bytes());
    n
}

// ---------------------------------------------------------------------------
// In-memory file table (static, no heap)
// ---------------------------------------------------------------------------

const MAX_FILES:       usize = 16;
const FILE_DATA_BYTES: usize = 16; // 16 bytes = two u64 message registers

static mut FILE_KEYS: [u64;                    MAX_FILES] = [0;         MAX_FILES];
static mut FILE_TAGS: [[u8; 16];               MAX_FILES] = [[0u8; 16]; MAX_FILES];
static mut FILE_DATA: [[u8; FILE_DATA_BYTES];  MAX_FILES] = [[0u8; 16]; MAX_FILES];
static mut FILE_USED: [bool;                   MAX_FILES] = [false;     MAX_FILES];

/// Return the index of the file with the given key, if present.
unsafe fn find_file(key: u64) -> Option<usize> {
    for i in 0..MAX_FILES {
        if FILE_USED[i] && FILE_KEYS[i] == key {
            return Some(i);
        }
    }
    None
}

/// Allocate a free slot for a new file, initialising its key.
unsafe fn alloc_slot(key: u64) -> Option<usize> {
    for i in 0..MAX_FILES {
        if !FILE_USED[i] {
            FILE_KEYS[i] = key;
            FILE_USED[i] = true;
            return Some(i);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// VFS operations
// ---------------------------------------------------------------------------

/// Encrypt `(data_lo, data_hi)` and store under `key`.
/// Returns 0 on success or an ERR_* code on failure.
unsafe fn vfs_write(key: u64, data_lo: u64, data_hi: u64) -> u64 {
    // Find existing slot or allocate a new one.
    let slot = match find_file(key) {
        Some(s) => s,
        None    => match alloc_slot(key) {
            Some(s) => s,
            None    => return ERR_NO_SPACE,
        },
    };

    // Pack the two u64 message registers into a 16-byte buffer.
    let mut buf = [0u8; FILE_DATA_BYTES];
    buf[..8].copy_from_slice(&data_lo.to_le_bytes());
    buf[8..].copy_from_slice(&data_hi.to_le_bytes());

    let cipher     = ChaCha20Poly1305::new(Key::from_slice(&MASTER_KEY));
    let nonce_arr  = file_nonce(key);
    let nonce      = Nonce::from_slice(&nonce_arr);

    match cipher.encrypt_in_place_detached(nonce, b"", &mut buf) {
        Ok(tag) => {
            FILE_DATA[slot] = buf;
            FILE_TAGS[slot].copy_from_slice(tag.as_ref());
            0 // success
        }
        Err(_) => ERR_CRYPTO,
    }
}

/// Decrypt and return the file stored under `key`.
/// Returns `(error_code, data_lo, data_hi)` — error_code 0 means success.
unsafe fn vfs_read(key: u64) -> (u64, u64, u64) {
    let slot = match find_file(key) {
        Some(s) => s,
        None    => return (ERR_NOT_FOUND, 0, 0),
    };

    let mut buf    = FILE_DATA[slot];
    let cipher     = ChaCha20Poly1305::new(Key::from_slice(&MASTER_KEY));
    let nonce_arr  = file_nonce(key);
    let nonce      = Nonce::from_slice(&nonce_arr);

    // Reconstruct the tag as a GenericArray reference.
    use chacha20poly1305::aead::generic_array::GenericArray;
    let tag_ga = GenericArray::from_slice(&FILE_TAGS[slot]);

    match cipher.decrypt_in_place_detached(nonce, b"", &mut buf, tag_ga) {
        Ok(()) => {
            let lo = u64::from_le_bytes(buf[..8].try_into().unwrap());
            let hi = u64::from_le_bytes(buf[8..].try_into().unwrap());
            (0, lo, hi)
        }
        Err(_) => (ERR_CRYPTO, 0, 0),
    }
}

// ---------------------------------------------------------------------------
// Message info helper
// ---------------------------------------------------------------------------

#[inline(always)]
fn msg_info(label: u64, length: u64) -> u64 {
    (label << 12) | (length & 0x7f)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Block until a client sends a file-operation request.
        let (info, _badge, mr0, mr1, mr2, _mr3) =
            unsafe { native::sel4_recv(SLOT_VFS_EP) };

        let label = info >> 12;

        match label {
            // WRITE: MR0=key, MR1=data_lo, MR2=data_hi
            MSG_WRITE_BLOCK => {
                let err = unsafe { vfs_write(mr0, mr1, mr2) };
                if err == 0 {
                    unsafe { native::sel4_reply(msg_info(MSG_OK, 0), 0, 0) };
                } else {
                    unsafe { native::sel4_reply(msg_info(MSG_ERROR, 1), err, 0) };
                }
            }

            // READ: MR0=key
            MSG_READ_BLOCK => {
                let (err, lo, hi) = unsafe { vfs_read(mr0) };
                if err == 0 {
                    unsafe { native::sel4_reply(msg_info(MSG_OK, 2), lo, hi) };
                } else {
                    unsafe { native::sel4_reply(msg_info(MSG_ERROR, 1), err, 0) };
                }
            }

            // Unknown operation — reply with generic error.
            _ => {
                unsafe { native::sel4_reply(msg_info(MSG_ERROR, 1), MSG_ERROR, 0) };
            }
        }
    }
}
