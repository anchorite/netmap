//! # Overview
//!
//! **Warning**: This is experimental project, it is unstable, probably unuseable. It's aim is to
//! try out stuff and allow me to learn Rust.
//!
//! This library provides Rust API to NetMap's libnetmap library.
use std::ops::Index;
use std::time::Duration;

// TODO:
// 1. Allow polling of multiple Ports

use netmap_sys::{
    netmap_buf_from_ring_slot, netmap_ring, netmap_ring_empty, netmap_rxring, netmap_slot,
    netmap_slot_from_ring, netmap_txring, nmport_close, nmport_d, nmport_open,
};

pub struct PortSpec {
    spec: String,
}

pub struct Port {
    port: *mut nmport_d,
    fd: i32,
    rx_rings: Vec<Ring>,
    tx_rings: Vec<Ring>,
    cur_rxring: usize,
}

impl PortSpec {
    pub fn from(spec: &str) -> Self {
        let spec = String::from(spec);

        Self { spec }
    }

    pub fn open_port(&self) -> Result<Port, String> {
        Port::open(&self.spec)
    }
}

impl Port {
    fn open(spec: &str) -> Result<Self, String> {
        let port = Port::open_port(spec)?;
        let tx_rings = Port::tx_rings_from_port(&port);
        let rx_rings = Port::rx_rings_from_port(&port);
        let fd = unsafe { (*port).fd };
        Ok(Self {
            fd,
            port,
            tx_rings,
            rx_rings,
            cur_rxring: 0,
        })
    }

    pub fn tx_rings(&self) -> &Vec<Ring> {
        &self.tx_rings
    }

    pub fn rx_rings(&self) -> &Vec<Ring> {
        &self.rx_rings
    }

    pub fn poll(&mut self, duration: Option<Duration>) -> bool {
        self.reset_before_poll();
        let pollfd = filedescriptor::pollfd {
            fd: self.fd,
            events: filedescriptor::POLLIN,
            revents: 0,
        };
        // TODO: process error
        filedescriptor::poll(&mut [pollfd], duration).unwrap() > 0
    }

    pub fn get_rx_slot(&mut self) -> Option<&Slot> {
        self.get_cur_rx_slot()
    }

    fn reset_before_poll(&mut self) {
        self.cur_rxring = 0;
    }

    fn get_cur_rx_slot(&mut self) -> Option<&Slot> {
        let ring = self.find_non_empty_ring()?;
        ring.get_next_slot()
    }

    fn find_non_empty_ring(&mut self) -> Option<&mut Ring> {
        self.cur_rxring = self.rx_rings[self.cur_rxring..]
            .iter()
            .position(|r| !r.is_empty())?;
        Some(&mut self.rx_rings[self.cur_rxring])
    }

    fn open_port(spec: &str) -> Result<*mut nmport_d, String> {
        let port = unsafe { nmport_open(spec.as_ptr() as *const i8) };
        if port.is_null() {
            Err(format!("Failed to open: {}", spec))
        } else {
            Ok(port)
        }
    }

    fn tx_rings_from_port(port: &*mut nmport_d) -> Vec<Ring> {
        unsafe { (**port).first_tx_ring..=(**port).last_tx_ring }
            .map(|ri| Port::txring_from_index(port, ri))
            .collect()
    }

    fn rx_rings_from_port(port: &*mut nmport_d) -> Vec<Ring> {
        unsafe { (**port).first_rx_ring..=(**port).last_rx_ring }
            .map(|ri| Port::rxring_from_index(port, ri))
            .collect()
    }

    fn txring_from_index(port: &*mut nmport_d, index: u16) -> Ring {
        let ring = unsafe { netmap_txring((**port).nifp, index) };
        Ring::new(RingType::Tx, ring, index)
    }

    fn rxring_from_index(port: &*mut nmport_d, index: u16) -> Ring {
        let ring = unsafe { netmap_rxring((**port).nifp, index) };
        Ring::new(RingType::Rx, ring, index)
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        unsafe { nmport_close(self.port) }
    }
}

enum RingType {
    Rx,
    Tx,
}

pub struct Ring {
    index: u16,
    ring_type: RingType,
    ring: *mut netmap_ring,
    slots: Vec<Slot>,
    head: usize,
    cur: usize,
    tail: usize,
}

impl Ring {
    fn new(ring_type: RingType, ring: *mut netmap_ring, index: u16) -> Self {
        let slots = Ring::create_slots(ring);
        let (head, cur, tail) = Ring::get_positions(ring);
        Self {
            index,
            ring,
            ring_type,
            slots,
            head,
            cur,
            tail,
        }
    }

    fn create_slots(ring: *mut netmap_ring) -> Vec<Slot> {
        let num_slots = unsafe { (*ring).num_slots };
        (0..num_slots)
            .into_iter()
            .map(|i| Slot::new(i as u16, ring))
            .collect()
    }

    fn get_positions(ring: *mut netmap_ring) -> (usize, usize, usize) {
        unsafe {
            (
                (*ring).head as usize,
                (*ring).cur as usize,
                (*ring).tail as usize,
            )
        }
    }

    pub fn iter(&self) -> impl Iterator + '_ {
        let slice = self.slots.as_slice();
        if self.head < self.tail {
            slice[self.head..self.tail].iter().chain(slice[0..0].iter())
        } else {
            slice[self.head..].iter().chain(slice[..self.tail].iter())
        }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { netmap_ring_empty(self.ring) }
    }

    pub fn get_next_slot(&mut self) -> Option<&Slot> {
        if self.is_empty() {
            None
        } else {
            let slot_ndx = self.head();
            self.advance_next_slot();
            Some(self.at(slot_ndx))
        }
    }

    pub fn at(&self, index: usize) -> &Slot {
        &self.slots[index]
    }

    fn head(&self) -> usize {
        unsafe { (*self.ring).head as usize }
    }

    fn advance_next_slot(&mut self) {
        let num_slots = unsafe { (*self.ring).num_slots as usize };
        let next_head = Ring::next_head(self.head(), num_slots);
        unsafe {
            (*self.ring).head = next_head as u32;
            (*self.ring).cur = next_head as u32;
        };
    }

    fn next_head(head: usize, num_slots: usize) -> usize {
        if head + 1 >= num_slots {
            0
        } else {
            head + 1
        }
    }

    fn valid_index(&self, index: usize) -> bool {
        if self.head < self.tail {
            self.head <= index && index < self.tail
        } else {
            (self.head <= index && index < self.slots.len()) || index < self.tail
        }
    }
}

impl Index<usize> for Ring {
    type Output = Slot;

    fn index(&self, index: usize) -> &Self::Output {
        if !self.valid_index(index) {
            panic!(
                "Invalid index: {} for [{}, {}]",
                index, self.head, self.tail
            )
        }
        &self.slots[index]
    }
}

pub struct Slot {
    index: u16,
    ring: *mut netmap_ring,
    slot: *mut netmap_slot,
}

impl Slot {
    fn new(index: u16, ring: *mut netmap_ring) -> Self {
        let slot = unsafe { netmap_slot_from_ring(ring, index) };
        Self { index, ring, slot }
    }

    // TODO: decide whether the slice should be the buf len or the slot len
    pub fn as_slice(&self) -> &[u8] {
        let buf = unsafe { netmap_buf_from_ring_slot(self.ring, self.slot) };
        unsafe { std::slice::from_raw_parts(buf, self.len()) }
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.slot).len as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<[u8]> for Slot {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
