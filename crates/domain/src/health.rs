#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Ok,
}

impl HealthStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HealthStatus;

    #[test]
    fn ok_status_serializes_to_ok_literal() {
        assert_eq!(HealthStatus::Ok.as_str(), "ok");
    }
}
