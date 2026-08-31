//! Shared GPUI event handler types.

use std::rc::Rc;

use gpui::{App, Window};

pub type WindowAppHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Reports the welcome brand image center and height in window coordinates.
pub type BrandLayoutTracker = Rc<dyn Fn(f32, f32, f32, &mut App)>;
