//! Shared callback type aliases for sidebar row interactions.

use std::rc::Rc;

use gpui::{App, Window};

pub type ThreadIdCallback = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub type ThreadSelectCallback = Rc<dyn Fn(String, bool, &mut Window, &mut App)>;
pub type ThreadHoverCallback = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;
pub type ThreadMoveCallback = Rc<dyn Fn(String, isize, &mut Window, &mut App)>;
pub type ThreadDragOverCallback = Rc<dyn Fn(String, bool, &mut Window, &mut App)>;
pub type ThreadDropCallback = Rc<dyn Fn(String, String, &mut Window, &mut App)>;
