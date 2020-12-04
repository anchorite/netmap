use netmap_sys::{nmport_d, nmport_open};

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

    pub fn open_port(&self) -> Port {
        Port::open(&self.spec)
    }
}

impl Port {
    fn open(spec: &String) -> Self {
        let port = unsafe { nmport_open(spec.as_ptr() as *const i8) };
        Self { port }
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
