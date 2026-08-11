//! Shared GPUI event handler types.

use std::rc::Rc;

use gpui::{App, Window};

pub type WindowAppHandler = Rc<dyn Fn(&mut Window, &mut App)>;
