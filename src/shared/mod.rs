//! Cross-cutting modules shared across the application.
//!
//! Uses the **Facade** pattern: a small public surface (`preferences`, `theme`) hides
//! serialization, storage strategy, and token resolution details.

pub mod preferences;
pub mod theme;
