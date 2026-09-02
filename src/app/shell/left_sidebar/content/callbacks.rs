//! Shared callback type aliases for sidebar row interactions.

use std::rc::Rc;

use gpui::{App, Window};

pub type AtomIdCallback = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub type AtomSelectCallback = Rc<dyn Fn(String, bool, &mut Window, &mut App)>;
pub type AtomHoverCallback = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;
pub type AtomMoveCallback = Rc<dyn Fn(String, isize, &mut Window, &mut App)>;
pub type AtomDragOverCallback = Rc<dyn Fn(String, bool, &mut Window, &mut App)>;
pub type AtomDropCallback = Rc<dyn Fn(String, String, &mut Window, &mut App)>;
