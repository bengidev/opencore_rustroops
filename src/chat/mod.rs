//! Chat UI, thread state, and SQLite persistence.

mod chat_state;
mod chat_store;
mod chat_view;
mod composer_actions;
mod composer_toolbar;
mod context_window_ring;
mod conversation_picker;
mod credential_dialog;
mod credential_ui;
mod credentials_banner;
mod generation_ui;
mod instructions_dialog;
mod markdown_render;
mod model_catalog_store;
mod model_picker;
mod stream_indicator;

pub use chat_state::*;
pub use chat_store::*;
pub use chat_view::ChatView;
pub use model_catalog_store::*;
