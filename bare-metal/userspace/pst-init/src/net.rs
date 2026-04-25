use alloc::string::String;
use alloc::vec;
use sel4_sys::*;
use crate::{serial_print, serial_print_num, serial_print_hex};
use pst_blk::virtio::*;

const VIRTIO_NET_DEVICE_LEGACY: u16 = 0x1000;

// Virtio net config offsets (after common registers at +0x14)
const REG_NET_MAC: u16 = 0x14;

// Virtqueue indices
const RX_QUEUE: u16 = 0;
const TX_QUEUE: u16 = 1;

#[repr(C, align(4096))]
struct NetPage([u8; 4096]);

static mut RX_QUEUE_BUF: NetPage = NetPage([0u8; 4096]);
static mut TX_QUEUE_BUF: NetPage = NetPage([0u8; 4096]);
static mut RX_PACKET_BUF: NetPage = NetPage([0u8; 4096]);
static mut TX_PACKET_BUF: NetPage = NetPage([0u8; 4096]);

pub struct VirtioNet {
    port_cap: u64,
    base_port: u16,
    mac: [u8; 6],
    rx_last_used: u16,
    tx_last_used: u16,
}

pub fn setup(pci_cap: u64) -> Option<VirtioNet> {
    let mut net_bar: u64 = 0;

    for dev in 0u8..32 {
        let addr: u32 = (1u32 << 31) | ((dev as u32) << 11);
        unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr); }
        let id = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
        if id == 0xFFFF_FFFF || id == 0 { continue; }

        let vendor = (id & 0xFFFF) as u16;
        let device = ((id >> 16) & 0xFFFF) as u16;

        if vendor == VIRTIO_VENDOR && device == VIRTIO_NET_DEVICE_LEGACY {
            unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr | 0x10); }
            let bar0 = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
            if bar0 & 1 == 1 {
                net_bar = (bar0 & 0xFFFFFFFC) as u64;
                serial_print("[net] virtio-net at slot ");
                serial_print_num(dev as usize);
                serial_print(", port=0x");
                serial_print_hex(net_bar);
                serial_print("\n");
                break;
            }
        }
    }

    if net_bar == 0 {
        serial_print("[net] No virtio-net found\n");
        return None;
    }

    let base_port = net_bar as u16;

    // Issue port cap
    // We need to find a free slot — use a high slot number
    // The caller should pass next_slot, but for simplicity we'll issue via IOPortControl
    // This won't work without a free slot... let's take a different approach
    // and have the caller pass the port cap
    // For now, issue directly — the caller manages slot allocation

    // Actually, we need next_slot. Let's restructure to take it.
    // For now, return the bar and let the caller do port cap issuance.

    None // Placeholder — see setup_with_slot below
}

pub fn setup_with_port(pci_cap: u64, mut next_slot: u64) -> (Option<VirtioNet>, u64) {
    let mut net_bar: u64 = 0;

    for dev in 0u8..32 {
        let addr: u32 = (1u32 << 31) | ((dev as u32) << 11);
        unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr); }
        let id = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
        if id == 0xFFFF_FFFF || id == 0 { continue; }

        let vendor = (id & 0xFFFF) as u16;
        let device = ((id >> 16) & 0xFFFF) as u16;

        if vendor == VIRTIO_VENDOR && device == VIRTIO_NET_DEVICE_LEGACY {
            unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr | 0x10); }
            let bar0 = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
            if bar0 & 1 == 1 {
                net_bar = (bar0 & 0xFFFFFFFC) as u64;
                serial_print("[net] virtio-net at slot ");
                serial_print_num(dev as usize);
                serial_print(", port=0x");
                serial_print_hex(net_bar);
                serial_print("\n");
                break;
            }
        }
    }

    if net_bar == 0 {
        serial_print("[net] No virtio-net found (add -device virtio-net-pci to QEMU)\n");
        return (None, next_slot);
    }

    let base_port = net_bar as u16;

    let port_cap = next_slot;
    next_slot += 1;
    let err = unsafe {
        seL4_X86_IOPortControl_Issue(
            seL4_CapIOPortControl, base_port, base_port + 0xFF,
            seL4_CapInitThreadCNode, port_cap, 64,
        )
    };
    if err != seL4_NoError {
        serial_print("[net] Port cap failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return (None, next_slot);
    }

    // Reset and initialize
    port_out8(port_cap, base_port + REG_DEVICE_STATUS, 0);
    port_out8(port_cap, base_port + REG_DEVICE_STATUS, STATUS_ACKNOWLEDGE);
    port_out8(port_cap, base_port + REG_DEVICE_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

    let _features = port_in32(port_cap, base_port + REG_DEVICE_FEATURES);
    port_out32(port_cap, base_port + REG_GUEST_FEATURES, 0);

    // Read MAC address
    let mut mac = [0u8; 6];
    for i in 0..6 {
        mac[i] = port_in8(port_cap, base_port + REG_NET_MAC + i as u16);
    }
    serial_print("[net] MAC: ");
    for (i, b) in mac.iter().enumerate() {
        if i > 0 { serial_print(":"); }
        serial_print_hex(*b as u64);
    }
    serial_print("\n");

    // Set up RX queue (queue 0)
    port_out16(port_cap, base_port + REG_QUEUE_SELECT, RX_QUEUE);
    let rx_qsize = port_in16(port_cap, base_port + REG_QUEUE_SIZE);
    serial_print("[net] RX queue size: ");
    serial_print_num(rx_qsize as usize);
    serial_print("\n");

    let rx_queue_vaddr = unsafe { &raw mut RX_QUEUE_BUF.0 as u64 };
    unsafe { core::ptr::write_bytes(rx_queue_vaddr as *mut u8, 0, 4096); }
    port_out32(port_cap, base_port + REG_QUEUE_ADDRESS, (rx_queue_vaddr >> 12) as u32);

    // Post RX buffer to receive queue
    let rx_buf_vaddr = unsafe { &raw mut RX_PACKET_BUF.0 as u64 };
    unsafe {
        let desc = rx_queue_vaddr as *mut VirtqDesc;
        (*desc.add(0)).addr = rx_buf_vaddr;
        (*desc.add(0)).len = 4096;
        (*desc.add(0)).flags = VIRTQ_DESC_F_WRITE;
        (*desc.add(0)).next = 0;

        let avail = (rx_queue_vaddr + 256) as *mut VirtqAvail;
        (*avail).ring[0] = 0;
        (*avail).idx = 1;
    }
    port_out16(port_cap, base_port + REG_QUEUE_NOTIFY, RX_QUEUE);

    // Set up TX queue (queue 1)
    port_out16(port_cap, base_port + REG_QUEUE_SELECT, TX_QUEUE);
    let tx_qsize = port_in16(port_cap, base_port + REG_QUEUE_SIZE);

    let tx_queue_vaddr = unsafe { &raw mut TX_QUEUE_BUF.0 as u64 };
    unsafe { core::ptr::write_bytes(tx_queue_vaddr as *mut u8, 0, 4096); }
    port_out32(port_cap, base_port + REG_QUEUE_ADDRESS, (tx_queue_vaddr >> 12) as u32);

    // Mark driver ready
    port_out8(port_cap, base_port + REG_DEVICE_STATUS,
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK);

    serial_print("[net] virtio-net ready\n");

    (Some(VirtioNet {
        port_cap, base_port, mac,
        rx_last_used: 0, tx_last_used: 0,
    }), next_slot)
}

impl VirtioNet {
    pub fn mac(&self) -> [u8; 6] { self.mac }

    pub fn send(&mut self, packet: &[u8]) -> bool {
        if packet.len() > 1514 { return false; }

        let tx_queue_vaddr = unsafe { &raw mut TX_QUEUE_BUF.0 as u64 };
        let tx_buf_vaddr = unsafe { &raw mut TX_PACKET_BUF.0 as u64 };

        unsafe {
            // Virtio net header (10 bytes for legacy) + packet data
            let buf = tx_buf_vaddr as *mut u8;
            core::ptr::write_bytes(buf, 0, 10); // zero virtio header
            core::ptr::copy_nonoverlapping(packet.as_ptr(), buf.add(10), packet.len());

            let desc = tx_queue_vaddr as *mut VirtqDesc;
            (*desc.add(0)).addr = tx_buf_vaddr;
            (*desc.add(0)).len = (10 + packet.len()) as u32;
            (*desc.add(0)).flags = 0; // device reads
            (*desc.add(0)).next = 0;

            let avail = (tx_queue_vaddr + 256) as *mut VirtqAvail;
            let idx = (*avail).idx;
            (*avail).ring[(idx % 16) as usize] = 0;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            (*avail).idx = idx.wrapping_add(1);
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        }

        port_out16(self.port_cap, self.base_port + REG_QUEUE_NOTIFY, TX_QUEUE);

        // Wait for TX completion
        let tx_queue_vaddr = unsafe { &raw mut TX_QUEUE_BUF.0 as u64 };
        unsafe {
            let used = (tx_queue_vaddr + 4096 - 256) as *mut VirtqUsed;
            let mut spins = 0u32;
            while core::ptr::read_volatile(&(*used).idx) == self.tx_last_used {
                core::hint::spin_loop();
                spins += 1;
                if spins > 1_000_000 { return false; }
            }
            self.tx_last_used = (*used).idx;
        }
        true
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> Option<usize> {
        let rx_queue_vaddr = unsafe { &raw mut RX_QUEUE_BUF.0 as u64 };
        let rx_buf_vaddr = unsafe { &raw mut RX_PACKET_BUF.0 as u64 };

        unsafe {
            let used = (rx_queue_vaddr + 4096 - 256) as *mut VirtqUsed;
            if core::ptr::read_volatile(&(*used).idx) == self.rx_last_used {
                return None; // no packet
            }

            let used_elem = &(*used).ring[(self.rx_last_used % 16) as usize];
            let total_len = used_elem.len as usize;
            self.rx_last_used = core::ptr::read_volatile(&(*used).idx);

            // Skip 10-byte virtio header
            if total_len <= 10 { return None; }
            let pkt_len = (total_len - 10).min(buf.len());
            let src = (rx_buf_vaddr as *const u8).add(10);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), pkt_len);

            // Re-post RX buffer
            let desc = rx_queue_vaddr as *mut VirtqDesc;
            (*desc.add(0)).addr = rx_buf_vaddr;
            (*desc.add(0)).len = 4096;
            (*desc.add(0)).flags = VIRTQ_DESC_F_WRITE;
            (*desc.add(0)).next = 0;

            let avail = (rx_queue_vaddr + 256) as *mut VirtqAvail;
            let idx = (*avail).idx;
            (*avail).ring[(idx % 16) as usize] = 0;
            (*avail).idx = idx.wrapping_add(1);
            port_out16(self.port_cap, self.base_port + REG_QUEUE_NOTIFY, RX_QUEUE);

            Some(pkt_len)
        }
    }

    pub fn recv_blocking(&mut self, buf: &mut [u8], timeout_spins: u32) -> Option<usize> {
        for _ in 0..timeout_spins {
            if let Some(n) = self.recv(buf) { return Some(n); }
            core::hint::spin_loop();
        }
        None
    }
}

// Minimal HTTP GET over raw TCP (using smoltcp)
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, IpAddress};

struct SmolDevice<'a> {
    net: &'a mut VirtioNet,
}

impl<'a> Device for SmolDevice<'a> {
    type RxToken<'b> = SmolRxToken where Self: 'b;
    type TxToken<'b> = SmolTxToken<'b> where Self: 'b;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut buf = vec![0u8; 1600];
        if let Some(len) = self.net.recv(&mut buf) {
            buf.truncate(len);
            Some((SmolRxToken { buf }, SmolTxToken { net: self.net }))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(SmolTxToken { net: self.net })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1514;
        caps
    }
}

struct SmolRxToken { buf: alloc::vec::Vec<u8> }
struct SmolTxToken<'a> { net: &'a mut VirtioNet }

impl RxToken for SmolRxToken {
    fn consume<R, F>(self, f: F) -> R where F: FnOnce(&[u8]) -> R {
        f(&self.buf)
    }
}

impl<'a> TxToken for SmolTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.net.send(&buf);
        result
    }
}

pub fn http_get(net: &mut VirtioNet, host_ip: [u8; 4], port: u16, path: &str) -> Option<String> {
    let mac = net.mac();
    let ethernet_addr = EthernetAddress(mac);
    let ip_addr = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);

    let mut device = SmolDevice { net };
    let mut config = Config::new(ethernet_addr.into());
    let mut iface = Interface::new(config, &mut device, Instant::ZERO);
    iface.update_ip_addrs(|addrs| { addrs.push(ip_addr).ok(); });
    iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2)).ok();

    let tcp_rx = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; 2048]),
        tcp::SocketBuffer::new(vec![0u8; 2048]),
    );
    let mut sockets = SocketSet::new(vec![]);
    let tcp_handle = sockets.add(tcp_rx);

    // Connect
    let dest = (IpAddress::v4(host_ip[0], host_ip[1], host_ip[2], host_ip[3]), port);
    sockets.get_mut::<tcp::Socket>(tcp_handle).connect(iface.context(), dest, 49152).ok()?;

    let request = alloc::format!("GET {} HTTP/1.0\r\nHost: {}.{}.{}.{}\r\nConnection: close\r\n\r\n",
        path, host_ip[0], host_ip[1], host_ip[2], host_ip[3]);

    let mut sent = false;
    let mut response = alloc::vec::Vec::new();
    let mut polls = 0u32;

    loop {
        iface.poll(Instant::from_millis(polls as i64), &mut device, &mut sockets);

        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);

        if socket.can_send() && !sent {
            socket.send_slice(request.as_bytes()).ok();
            sent = true;
        }

        if socket.can_recv() {
            socket.recv(|data| {
                response.extend_from_slice(data);
                (data.len(), ())
            }).ok();
        }

        if !socket.is_open() && sent {
            break;
        }

        polls += 1;
        if polls > 5_000_000 { break; }

        // Re-borrow device for next poll
        // This won't compile as-is because of borrow conflicts...
        // smoltcp needs careful lifetime management
    }

    // Strip HTTP headers
    let text = String::from_utf8_lossy(&response).into_owned();
    if let Some(body_start) = text.find("\r\n\r\n") {
        Some(String::from(&text[body_start + 4..]))
    } else {
        Some(text)
    }
}

fn port_in8(cap: u64, port: u16) -> u8 { unsafe { native::sel4_ioport_in8(cap, port) } }
fn port_in16(cap: u64, port: u16) -> u16 { unsafe { native::sel4_ioport_in16(cap, port) } }
fn port_in32(cap: u64, port: u16) -> u32 { unsafe { native::sel4_ioport_in32(cap, port) } }
fn port_out8(cap: u64, port: u16, v: u8) { unsafe { native::sel4_ioport_out8(cap, port, v); } }
fn port_out16(cap: u64, port: u16, v: u16) { unsafe { native::sel4_ioport_out16(cap, port, v); } }
fn port_out32(cap: u64, port: u16, v: u32) { unsafe { native::sel4_ioport_out32(cap, port, v); } }
