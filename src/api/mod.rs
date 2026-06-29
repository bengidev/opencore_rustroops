//! Provider-agnostic API boundary for chat completions and model catalog.

mod chat_provider;
mod credential_store;
mod credentials;
mod http_runtime;
mod openrouter_client;
mod openrouter_provider;

pub use chat_provider::*;
pub use credential_store::*;
pub use credentials::*;
pub use http_runtime::spawn as spawn_http_task;
pub use openrouter_provider::{OpenRouterProvider, DEFAULT_MODEL};
