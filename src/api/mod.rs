//! Provider-agnostic API boundary for chat completions and model catalog.

mod chat_provider;
mod credential_store;
mod credentials;
mod http_runtime;
mod model_info;
mod openrouter_client;
mod openrouter_provider;
mod speed_mode;

pub use chat_provider::*;
pub use credential_store::*;
pub use credentials::*;
pub use http_runtime::spawn as spawn_http_task;
pub use model_info::{
    GATEWAY_REASONING_EFFORTS, ModelInfo, ReasoningCapabilities, filter_generation_for_model,
    generation_without_model_metadata,
};
pub use openrouter_provider::{DEFAULT_MODEL, OpenRouterProvider};
pub use speed_mode::{SpeedWireField, apply_speed_to_json, model_supports_speed_mode, speed_wire_field};
