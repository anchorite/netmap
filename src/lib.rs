use netmap_sys::{netmap_ring, netmap_rxring, netmap_txring, nmport_close, nmport_d, nmport_open};

pub struct PortSpec {
    spec: String,
}

pub struct Port {
    port: *mut nmport_d,
    rx_rings: Vec<Ring>,
    tx_rings: Vec<Ring>,
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
        Ok(Self {
            port,
            tx_rings,
            rx_rings,
        })
    }

    pub fn tx_rings(&self) -> &Vec<Ring> {
        &self.tx_rings
    }

    pub fn rx_rings(&self) -> &Vec<Ring> {
        &self.rx_rings
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
        Ring {
            index,
            ring,
            ring_type: RingType::Tx,
        }
    }

    fn rxring_from_index(port: &*mut nmport_d, index: u16) -> Ring {
        let ring = unsafe { netmap_rxring((**port).nifp, index) };
        Ring {
            index,
            ring,
            ring_type: RingType::Rx,
        }
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
}

impl Ring {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
