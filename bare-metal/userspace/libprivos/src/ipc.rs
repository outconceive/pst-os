// Safe IPC abstractions over seL4 endpoints.
//
// seL4 IPC is synchronous rendezvous: both sender and receiver must be
// ready before the message passes. This module wraps the raw seL4 calls
// in typed Rust abstractions.

use sel4_sys::*;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Endpoint
// ---------------------------------------------------------------------------

/// A typed seL4 IPC endpoint capability.
///
/// Endpoints are unforgeable tokens. A process can only send/receive on
/// an endpoint if it holds the capability — the kernel enforces this.
pub struct Endpoint {
    cap: seL4_CPtr,
}

impl Endpoint {
    /// Wrap a raw capability pointer given by the parent process.
    ///
    /// # Safety
    /// The caller must guarantee `cap` is a valid endpoint capability
    /// in the current thread's CSpace.
    pub unsafe fn from_cap(cap: seL4_CPtr) -> Self {
        Self { cap }
    }

    pub fn cap(&self) -> seL4_CPtr {
        self.cap
    }

    /// Send a message (blocks until the receiver is ready).
    pub fn send(&self, msg: &IpcMessage) {
        let word_count = msg.words.len().min(seL4_MsgMaxLength);
        let info = seL4_MessageInfo_t::new(
            msg.label,
            0,
            0,
            word_count as seL4_Word,
        );
        for (i, &word) in msg.words[..word_count].iter().enumerate() {
            unsafe { seL4_SetMR(i as i32, word) };
        }
        unsafe { seL4_Send(self.cap, info) };
    }

    /// Receive a message (blocks until a sender arrives).
    pub fn recv(&self) -> IpcMessage {
        let mut sender_badge: seL4_Word = 0;
        let info = unsafe { seL4_Recv(self.cap, &mut sender_badge) };
        let length = (info.get_length() as usize).min(seL4_MsgMaxLength);

        #[cfg(feature = "alloc")]
        let words = {
            let mut w = Vec::with_capacity(length);
            for i in 0..length {
                w.push(unsafe { seL4_GetMR(i as i32) });
            }
            w
        };

        IpcMessage {
            label: info.get_label(),
            words,
            sender_badge,
        }
    }

    /// Send then wait for a reply (call semantics).
    pub fn call(&self, msg: &IpcMessage) -> IpcMessage {
        let info = seL4_MessageInfo_t::new(
            msg.label,
            0,
            0,
            msg.words.len() as seL4_Word,
        );
        for (i, &word) in msg.words.iter().enumerate() {
            unsafe { seL4_SetMR(i as i32, word) };
        }
        let reply_info = unsafe { seL4_Call(self.cap, info) };
        let length = reply_info.get_length() as usize;

        #[cfg(feature = "alloc")]
        let words = {
            let mut w = Vec::with_capacity(length);
            for i in 0..length {
                w.push(unsafe { seL4_GetMR(i as i32) });
            }
            w
        };

        IpcMessage {
            label: reply_info.get_label(),
            words,
            sender_badge: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// IpcMessage
// ---------------------------------------------------------------------------

/// A message passed over seL4 IPC.
#[cfg(feature = "alloc")]
pub struct IpcMessage {
    pub label: seL4_Word,
    pub words: Vec<seL4_Word>,
    pub sender_badge: seL4_Word,
}

#[cfg(feature = "alloc")]
impl IpcMessage {
    pub fn new(label: seL4_Word) -> Self {
        Self { label, words: Vec::new(), sender_badge: 0 }
    }

    pub fn with_word(mut self, word: seL4_Word) -> Self {
        self.words.push(word);
        self
    }
}

// ---------------------------------------------------------------------------
// Well-known message labels (shared across all Privion IPC peers)
// ---------------------------------------------------------------------------

pub mod labels {
    use sel4_sys::seL4_Word;

    pub const MSG_OK: seL4_Word              = 0x0000;
    pub const MSG_ERROR: seL4_Word           = 0x0001;

    // VFS
    pub const MSG_READ_BLOCK: seL4_Word      = 0x0100;
    pub const MSG_WRITE_BLOCK: seL4_Word     = 0x0101;

    // Network daemon
    pub const MSG_DNS_RESOLVE: seL4_Word     = 0x0200;
    pub const MSG_TCP_CONNECT: seL4_Word     = 0x0201;
    pub const MSG_TCP_SEND: seL4_Word        = 0x0202;
    pub const MSG_TCP_RECV: seL4_Word        = 0x0203;

    // Driver manager
    pub const MSG_DRIVER_REGISTER: seL4_Word = 0x0300;
    pub const MSG_DRIVER_READY: seL4_Word    = 0x0301;
}
