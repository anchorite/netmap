use netmap_sys::{nmport_close, nmport_d, nmport_open};

pub struct PortSpec {
    spec: String,
}

pub struct Port {
    port: *mut nmport_d,
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
        Ok(Self { port })
    }

    fn open_port(spec: &str) -> Result<*mut nmport_d, String> {
        let port = unsafe { nmport_open(spec.as_ptr() as *const i8) };
        if port.is_null() {
            Err(format!("Failed to open: {}", spec))
        } else {
            Ok(port)
        }
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        unsafe { nmport_close(self.port) }
    }
}

struct Ring {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
