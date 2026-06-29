//! Environment and saved credential resolution for OpenRouter.

use super::chat_provider::{CredentialSource, CredentialStatus};
use super::credential_store::CredentialStore;

const OPENROUTER_API_KEY: &str = "OPENROUTER_API_KEY";
const OPENROUTER_KEY: &str = "OPENROUTER_KEY";

/// Resolves an OpenRouter API key from the process environment.
///
/// `OPENROUTER_API_KEY` takes precedence over `OPENROUTER_KEY`.
pub fn resolve_openrouter_api_key_from_env() -> Option<String> {
    std::env::var(OPENROUTER_API_KEY)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var(OPENROUTER_KEY)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

/// Resolves an OpenRouter API key from environment variables, then saved storage.
///
/// Environment credentials always win over a saved key.
pub fn resolve_openrouter_api_key(store: &dyn CredentialStore) -> Option<String> {
    resolve_openrouter_api_key_from_env().or_else(|| store.saved_api_key())
}

/// Returns credential availability for the OpenRouter provider.
pub fn openrouter_credential_status(store: &dyn CredentialStore) -> CredentialStatus {
    if resolve_openrouter_api_key_from_env().is_some() {
        return CredentialStatus::Available {
            source: CredentialSource::Environment,
        };
    }

    if store.saved_api_key().is_some() {
        return CredentialStatus::Available {
            source: CredentialSource::Saved,
        };
    }

    CredentialStatus::Missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::credential_store::InMemoryCredentialStore;
    use std::collections::HashMap;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        previous: HashMap<&'static str, Option<String>>,
    }

    impl EnvGuard {
        fn with(values: &[(&'static str, Option<&str>)]) -> Self {
            let lock = ENV_LOCK.lock().expect("env lock");
            let mut previous = HashMap::new();
            for (key, value) in values {
                previous.insert(*key, std::env::var(key).ok());
                match value {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.previous {
                match value {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[test]
    fn openrouter_api_key_takes_precedence_over_openrouter_key() {
        let _env = EnvGuard::with(&[
            (OPENROUTER_API_KEY, Some("from-api-key")),
            (OPENROUTER_KEY, Some("from-key")),
        ]);
        assert_eq!(
            resolve_openrouter_api_key_from_env().as_deref(),
            Some("from-api-key")
        );
    }

    #[test]
    fn openrouter_key_is_used_when_api_key_is_missing() {
        let _env = EnvGuard::with(&[
            (OPENROUTER_API_KEY, None),
            (OPENROUTER_KEY, Some("from-key")),
        ]);
        assert_eq!(
            resolve_openrouter_api_key_from_env().as_deref(),
            Some("from-key")
        );
    }

    #[test]
    fn empty_env_values_are_ignored() {
        let _env = EnvGuard::with(&[
            (OPENROUTER_API_KEY, Some("   ")),
            (OPENROUTER_KEY, Some("from-key")),
        ]);
        assert_eq!(
            resolve_openrouter_api_key_from_env().as_deref(),
            Some("from-key")
        );
    }

    #[test]
    fn credential_status_is_missing_without_env_or_saved_key() {
        let _env = EnvGuard::with(&[(OPENROUTER_API_KEY, None), (OPENROUTER_KEY, None)]);
        let store = InMemoryCredentialStore::new();
        assert_eq!(openrouter_credential_status(&store), CredentialStatus::Missing);
    }

    #[test]
    fn credential_status_reports_environment_source() {
        let _env = EnvGuard::with(&[
            (OPENROUTER_API_KEY, Some("secret")),
            (OPENROUTER_KEY, None),
        ]);
        let store = InMemoryCredentialStore::new();
        store.save_api_key("saved").expect("save");
        assert_eq!(
            openrouter_credential_status(&store),
            CredentialStatus::Available {
                source: CredentialSource::Environment
            }
        );
    }

    #[test]
    fn saved_key_is_used_when_env_is_missing() {
        let _env = EnvGuard::with(&[(OPENROUTER_API_KEY, None), (OPENROUTER_KEY, None)]);
        let store = InMemoryCredentialStore::new();
        store.save_api_key("saved-key").expect("save");
        assert_eq!(
            resolve_openrouter_api_key(&store).as_deref(),
            Some("saved-key")
        );
    }

    #[test]
    fn env_api_key_wins_over_saved_key() {
        let _env = EnvGuard::with(&[(OPENROUTER_API_KEY, Some("env-key")), (OPENROUTER_KEY, None)]);
        let store = InMemoryCredentialStore::new();
        store.save_api_key("saved-key").expect("save");
        assert_eq!(
            resolve_openrouter_api_key(&store).as_deref(),
            Some("env-key")
        );
    }

    #[test]
    fn credential_status_reports_saved_source() {
        let _env = EnvGuard::with(&[(OPENROUTER_API_KEY, None), (OPENROUTER_KEY, None)]);
        let store = InMemoryCredentialStore::new();
        store.save_api_key("saved").expect("save");
        assert_eq!(
            openrouter_credential_status(&store),
            CredentialStatus::Available {
                source: CredentialSource::Saved
            }
        );
    }
}
