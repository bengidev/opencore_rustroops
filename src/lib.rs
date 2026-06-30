#![recursion_limit = "256"] // gpui-component macro expansion depth needs >128
//! opencore_rustroops library surface.
//!
//! Public modules: [`app`] (composition root), [`api`] (provider boundary),
//! [`chat`] (chat UI and persistence), and [`shared`] (theme and preferences).

pub mod api;
pub mod app;
pub mod chat;
pub mod shared;
