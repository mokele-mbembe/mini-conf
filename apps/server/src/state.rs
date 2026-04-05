use infra::AppIdentity;

#[derive(Debug, Clone, Copy)]
pub struct AppState {
    identity: AppIdentity,
}

impl AppState {
    pub const fn new(identity: AppIdentity) -> Self {
        Self { identity }
    }

    pub const fn identity(self) -> AppIdentity {
        self.identity
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use infra::AppIdentity;

    #[test]
    fn exposes_app_identity() {
        let state = AppState::new(AppIdentity::new("mini-conf-server", "0.1.0"));

        assert_eq!(
            state.identity(),
            AppIdentity::new("mini-conf-server", "0.1.0")
        );
    }
}
