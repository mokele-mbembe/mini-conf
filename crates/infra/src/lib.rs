#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppIdentity {
    pub service_name: &'static str,
    pub version: &'static str,
}

impl AppIdentity {
    pub const fn new(service_name: &'static str, version: &'static str) -> Self {
        Self {
            service_name,
            version,
        }
    }
}
