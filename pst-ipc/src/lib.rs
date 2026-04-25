#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use alloc::vec::Vec;

use libpst::table::ParallelTable;
use libpst::offset::OffsetTable;

// Message status
pub const STATUS_PENDING: u8   = b'P';
pub const STATUS_DELIVERED: u8 = b'D';
pub const STATUS_READ: u8      = b'R';

// Message priority
pub const PRIORITY_NORMAL: u8    = 0;
pub const PRIORITY_HIGH: u8      = 1;
pub const PRIORITY_INTERRUPT: u8 = 2;

// Column indices
const COL_SENDER: usize   = 0;
const COL_RECEIVER: usize = 1;
const COL_STATUS: usize   = 2;
const COL_PRIORITY: usize = 3;
const COL_CHANNEL: usize  = 4;

const MAX_PAYLOAD: usize = 4096;

#[derive(Debug)]
pub enum IpcError {
    PayloadTooLarge,
    NotFound,
    NotRecipient,
    AlreadyRead,
    NotDelivered,
}

pub struct Message {
    pub sender: u8,
    pub receiver: u8,
    pub channel: u8,
    pub priority: u8,
    pub payload: Vec<u8>,
}

pub struct EventLog {
    meta: ParallelTable,
    offsets: OffsetTable,
    payloads: Vec<Option<Vec<u8>>>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            meta: ParallelTable::new(&["sender", "receiver", "status", "priority", "channel"]),
            offsets: OffsetTable::new(),
            payloads: Vec::new(),
        }
    }

    pub fn send(&mut self, msg: Message) -> Result<usize, IpcError> {
        if msg.payload.len() > MAX_PAYLOAD {
            return Err(IpcError::PayloadTooLarge);
        }

        let physical = self.meta.append(&[
            msg.sender,
            msg.receiver,
            STATUS_PENDING,
            msg.priority,
            msg.channel,
        ]);
        let logical = self.offsets.assign(physical);

        while self.payloads.len() <= logical {
            self.payloads.push(None);
        }
        self.payloads[logical] = Some(msg.payload);

        Ok(logical)
    }

    /// Drain pending messages for a receiver, ordered by priority (highest first).
    pub fn recv(&mut self, receiver_id: u8) -> Vec<(usize, u8, Vec<u8>)> {
        let pending = self.meta.scan(COL_STATUS, |v| v == STATUS_PENDING);

        let mut messages: Vec<(usize, u8, u8, Vec<u8>)> = Vec::new();

        for phys in pending {
            if self.meta.get(COL_RECEIVER, phys) != Some(receiver_id) {
                continue;
            }

            // Find logical ID for this physical position
            let logical = match self.find_logical(phys) {
                Some(l) => l,
                None => continue,
            };

            let sender = self.meta.get(COL_SENDER, phys).unwrap_or(0);
            let priority = self.meta.get(COL_PRIORITY, phys).unwrap_or(0);
            let payload = self.payloads.get(logical)
                .and_then(|p| p.clone())
                .unwrap_or_default();

            // Mark as delivered
            self.meta.set(COL_STATUS, phys, STATUS_DELIVERED);

            messages.push((logical, sender, priority, payload));
        }

        // Sort by priority descending (interrupts first)
        messages.sort_by(|a, b| b.2.cmp(&a.2));

        messages.into_iter().map(|(id, sender, _, payload)| (id, sender, payload)).collect()
    }

    /// Acknowledge a message — marks it read, becomes a tombstone candidate.
    pub fn ack(&mut self, logical_id: usize) -> Result<(), IpcError> {
        let phys = self.offsets.resolve(logical_id).ok_or(IpcError::NotFound)?;

        match self.meta.get(COL_STATUS, phys) {
            Some(STATUS_READ) => return Err(IpcError::AlreadyRead),
            Some(STATUS_PENDING) => return Err(IpcError::NotDelivered),
            _ => {}
        }

        self.meta.set(COL_STATUS, phys, STATUS_READ);
        Ok(())
    }

    /// Tombstone all read messages — the GC sweep.
    pub fn gc(&mut self) {
        let read_positions = self.meta.scan(COL_STATUS, |v| v == STATUS_READ);
        for phys in read_positions {
            if let Some(logical) = self.find_logical(phys) {
                self.meta.tombstone(phys);
                self.offsets.invalidate(logical);
                if logical < self.payloads.len() {
                    self.payloads[logical] = None;
                }
            }
        }
    }

    /// Peek at pending count for a receiver without consuming.
    pub fn pending_count(&self, receiver_id: u8) -> usize {
        let pending = self.meta.scan(COL_STATUS, |v| v == STATUS_PENDING);
        pending.iter().filter(|&&phys| {
            self.meta.get(COL_RECEIVER, phys) == Some(receiver_id)
        }).count()
    }

    /// Broadcast — send to multiple receivers. Returns list of message IDs.
    pub fn broadcast(
        &mut self,
        sender: u8,
        receivers: &[u8],
        channel: u8,
        priority: u8,
        payload: &[u8],
    ) -> Result<Vec<usize>, IpcError> {
        if payload.len() > MAX_PAYLOAD {
            return Err(IpcError::PayloadTooLarge);
        }
        let mut ids = Vec::new();
        for &recv in receivers {
            let id = self.send(Message {
                sender,
                receiver: recv,
                channel,
                priority,
                payload: payload.to_vec(),
            })?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Scan messages on a specific channel.
    pub fn scan_channel(&self, channel: u8) -> Vec<usize> {
        let matches = self.meta.scan(COL_CHANNEL, |v| v == channel);
        let mut logicals = Vec::new();
        for phys in matches {
            if let Some(l) = self.find_logical(phys) {
                logicals.push(l);
            }
        }
        logicals
    }

    pub fn compact(&mut self) {
        let remap = self.meta.compact();
        self.offsets.rebuild_from_remap(&remap);
    }

    pub fn total_messages(&self) -> usize {
        self.meta.len()
    }

    pub fn live_messages(&self) -> usize {
        self.meta.live_count()
    }

    pub fn tombstone_count(&self) -> usize {
        self.meta.tombstone_count()
    }

    fn find_logical(&self, physical: usize) -> Option<usize> {
        self.offsets.reverse_lookup(physical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_and_recv() {
        let mut log = EventLog::new();
        log.send(Message {
            sender: 1,
            receiver: 2,
            channel: 0,
            priority: PRIORITY_NORMAL,
            payload: b"hello".to_vec(),
        }).unwrap();

        let msgs = log.recv(2);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].1, 1); // sender
        assert_eq!(msgs[0].2, b"hello");
    }

    #[test]
    fn test_recv_filters_by_receiver() {
        let mut log = EventLog::new();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: 0, payload: b"for-2".to_vec() }).unwrap();
        log.send(Message { sender: 1, receiver: 3, channel: 0, priority: 0, payload: b"for-3".to_vec() }).unwrap();

        let msgs = log.recv(2);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].2, b"for-2");

        let msgs = log.recv(3);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].2, b"for-3");
    }

    #[test]
    fn test_priority_ordering() {
        let mut log = EventLog::new();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: PRIORITY_NORMAL, payload: b"low".to_vec() }).unwrap();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: PRIORITY_INTERRUPT, payload: b"urgent".to_vec() }).unwrap();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: PRIORITY_HIGH, payload: b"high".to_vec() }).unwrap();

        let msgs = log.recv(2);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].2, b"urgent");
        assert_eq!(msgs[1].2, b"high");
        assert_eq!(msgs[2].2, b"low");
    }

    #[test]
    fn test_ack_and_gc() {
        let mut log = EventLog::new();
        let id = log.send(Message { sender: 1, receiver: 2, channel: 0, priority: 0, payload: b"data".to_vec() }).unwrap();

        let msgs = log.recv(2); // marks as delivered
        assert_eq!(msgs.len(), 1);

        log.ack(id).unwrap();
        log.gc();
        log.compact();

        assert_eq!(log.live_messages(), 0);
        assert_eq!(log.total_messages(), 0);
    }

    #[test]
    fn test_pending_count() {
        let mut log = EventLog::new();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: 0, payload: b"a".to_vec() }).unwrap();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: 0, payload: b"b".to_vec() }).unwrap();
        log.send(Message { sender: 1, receiver: 3, channel: 0, priority: 0, payload: b"c".to_vec() }).unwrap();

        assert_eq!(log.pending_count(2), 2);
        assert_eq!(log.pending_count(3), 1);
        assert_eq!(log.pending_count(99), 0);
    }

    #[test]
    fn test_broadcast() {
        let mut log = EventLog::new();
        let ids = log.broadcast(1, &[2, 3, 4], 0, PRIORITY_HIGH, b"alert").unwrap();
        assert_eq!(ids.len(), 3);

        assert_eq!(log.pending_count(2), 1);
        assert_eq!(log.pending_count(3), 1);
        assert_eq!(log.pending_count(4), 1);
    }

    #[test]
    fn test_channel_scan() {
        let mut log = EventLog::new();
        log.send(Message { sender: 1, receiver: 2, channel: 10, priority: 0, payload: b"ch10".to_vec() }).unwrap();
        log.send(Message { sender: 1, receiver: 2, channel: 20, priority: 0, payload: b"ch20".to_vec() }).unwrap();
        log.send(Message { sender: 3, receiver: 4, channel: 10, priority: 0, payload: b"ch10b".to_vec() }).unwrap();

        let ch10 = log.scan_channel(10);
        assert_eq!(ch10.len(), 2);

        let ch20 = log.scan_channel(20);
        assert_eq!(ch20.len(), 1);
    }

    #[test]
    fn test_double_recv_no_duplicate() {
        let mut log = EventLog::new();
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: 0, payload: b"once".to_vec() }).unwrap();

        let first = log.recv(2);
        assert_eq!(first.len(), 1);

        let second = log.recv(2);
        assert_eq!(second.len(), 0); // already delivered
    }

    #[test]
    fn test_gc_lifecycle() {
        let mut log = EventLog::new();

        // Send 5 messages
        for i in 0..5u8 {
            log.send(Message { sender: 1, receiver: 2, channel: 0, priority: 0, payload: alloc::vec![i] }).unwrap();
        }
        assert_eq!(log.total_messages(), 5);

        // Receive and ack all
        let msgs = log.recv(2);
        for (id, _, _) in &msgs {
            log.ack(*id).unwrap();
        }

        // GC should tombstone all
        log.gc();
        assert_eq!(log.live_messages(), 0);

        // Compact should reclaim
        log.compact();
        assert_eq!(log.total_messages(), 0);
    }

    #[test]
    fn test_interrupt_as_high_priority_append() {
        let mut log = EventLog::new();

        // Normal message already waiting
        log.send(Message { sender: 1, receiver: 2, channel: 0, priority: PRIORITY_NORMAL, payload: b"normal".to_vec() }).unwrap();

        // Hardware interrupt arrives
        log.send(Message { sender: 0, receiver: 2, channel: 0, priority: PRIORITY_INTERRUPT, payload: b"irq".to_vec() }).unwrap();

        // Receiver drains — interrupt comes first
        let msgs = log.recv(2);
        assert_eq!(msgs[0].2, b"irq");
        assert_eq!(msgs[1].2, b"normal");
    }
}
