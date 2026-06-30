//! Credential banner and cache state for the chat surface.

/// Tracks whether credentials are missing and whether the banner was dismissed.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CredentialUiState {
    pub missing: bool,
    pub banner_dismissed: bool,
}

impl CredentialUiState {
    pub fn should_show_banner(&self) -> bool {
        self.missing && !self.banner_dismissed
    }

    pub fn refresh(&mut self, was_missing: bool, now_missing: bool) {
        self.missing = now_missing;
        if !was_missing && now_missing {
            self.banner_dismissed = false;
        }
    }

    pub fn dismiss_banner(&mut self) {
        self.banner_dismissed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_hidden_when_credentials_available() {
        let state = CredentialUiState {
            missing: false,
            banner_dismissed: false,
        };
        assert!(!state.should_show_banner());
    }

    #[test]
    fn banner_shown_when_missing_and_not_dismissed() {
        let state = CredentialUiState {
            missing: true,
            banner_dismissed: false,
        };
        assert!(state.should_show_banner());
    }

    #[test]
    fn banner_hidden_after_dismiss() {
        let mut state = CredentialUiState {
            missing: true,
            banner_dismissed: false,
        };
        state.dismiss_banner();
        assert!(!state.should_show_banner());
    }

    #[test]
    fn banner_reappears_when_credentials_become_missing_again() {
        let mut state = CredentialUiState {
            missing: false,
            banner_dismissed: true,
        };
        state.refresh(false, true);
        assert!(state.should_show_banner());
    }
}
