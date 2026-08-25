//! Drive gpui-component dock clip sizes during show/hide tweens.

use std::time::Instant;

use gpui::{App, Context, Entity, Window, px};
use gpui_component::dock::{Dock, DockArea, DockPlacement};

use super::dock_tween::{DockSizeTween, eval_dock_tween};
use super::layout::ShellLayout;

#[derive(Debug, Default)]
pub struct DockTweenState {
    pub left: Option<DockSizeTween>,
    pub right: Option<DockSizeTween>,
    pub bottom: Option<DockSizeTween>,
}

impl DockTweenState {
    pub fn tween_mut(&mut self, placement: DockPlacement) -> &mut Option<DockSizeTween> {
        match placement {
            DockPlacement::Left => &mut self.left,
            DockPlacement::Right => &mut self.right,
            DockPlacement::Bottom => &mut self.bottom,
            DockPlacement::Center => unreachable!("center dock is not toggleable"),
        }
    }

    pub fn tween(&self, placement: DockPlacement) -> Option<DockSizeTween> {
        match placement {
            DockPlacement::Left => self.left,
            DockPlacement::Right => self.right,
            DockPlacement::Bottom => self.bottom,
            DockPlacement::Center => None,
        }
    }

    pub fn any_active(&self, now: Instant) -> bool {
        [
            DockPlacement::Left,
            DockPlacement::Right,
            DockPlacement::Bottom,
        ]
        .into_iter()
        .any(|placement| {
            self.tween(placement)
                .is_some_and(|tween| tween.is_active(now))
        })
    }
}

pub fn dock_entity(dock_area: &DockArea, placement: DockPlacement) -> Option<&Entity<Dock>> {
    match placement {
        DockPlacement::Left => dock_area.left_dock(),
        DockPlacement::Right => dock_area.right_dock(),
        DockPlacement::Bottom => dock_area.bottom_dock(),
        DockPlacement::Center => None,
    }
}

pub fn dock_rest_size(dock: &Dock) -> f32 {
    dock.size().as_f32()
}

pub fn dock_clip_size(dock: &Dock) -> f32 {
    dock.display_size().as_f32()
}

pub fn apply_dock_clip_size(
    dock_area: &Entity<DockArea>,
    placement: DockPlacement,
    size: f32,
    cx: &mut App,
) {
    let dock = match placement {
        DockPlacement::Left => dock_area.read(cx).left_dock().cloned(),
        DockPlacement::Right => dock_area.read(cx).right_dock().cloned(),
        DockPlacement::Bottom => dock_area.read(cx).bottom_dock().cloned(),
        DockPlacement::Center => None,
    };
    if let Some(dock) = dock {
        dock.update(cx, |dock, cx| {
            dock.set_animated_size(Some(px(size)), cx);
        });
    }
}

pub fn finish_dock_tween(
    dock_area: &Entity<DockArea>,
    placement: DockPlacement,
    tween: DockSizeTween,
    window: &mut Window,
    cx: &mut App,
) {
    let dock = match placement {
        DockPlacement::Left => dock_area.read(cx).left_dock().cloned(),
        DockPlacement::Right => dock_area.read(cx).right_dock().cloned(),
        DockPlacement::Bottom => dock_area.read(cx).bottom_dock().cloned(),
        DockPlacement::Center => None,
    };
    let Some(dock) = dock else {
        return;
    };

    if tween.to <= f32::EPSILON {
        dock.update(cx, |dock, cx| {
            dock.clear_animated_size(cx);
            dock.set_open(false, window, cx);
        });
        return;
    }

    dock.update(cx, |dock, cx| dock.clear_animated_size(cx));
}

/// Returns animated dock sizes and whether any tween is still mid-flight.
pub fn tick_dock_tweens(
    dock_area: &Entity<DockArea>,
    tweens: &mut DockTweenState,
    now: Instant,
    window: &mut Window,
    cx: &mut App,
) -> (f32, f32, f32, bool) {
    let mut needs_frame = false;
    let mut left = dock_display_size(dock_area, DockPlacement::Left, tweens.left, now, cx);
    let mut right = dock_display_size(dock_area, DockPlacement::Right, tweens.right, now, cx);
    let mut bottom = dock_display_size(dock_area, DockPlacement::Bottom, tweens.bottom, now, cx);

    for placement in [
        DockPlacement::Left,
        DockPlacement::Right,
        DockPlacement::Bottom,
    ] {
        let slot = tweens.tween_mut(placement);
        let Some(tween) = *slot else {
            continue;
        };

        let target = tween.to;
        let display = eval_dock_tween(Some(tween), target, now);
        apply_dock_clip_size(dock_area, placement, display, cx);

        match placement {
            DockPlacement::Left => left = display,
            DockPlacement::Right => right = display,
            DockPlacement::Bottom => bottom = display,
            DockPlacement::Center => {}
        }

        if tween.is_active(now) {
            needs_frame = true;
        } else {
            finish_dock_tween(dock_area, placement, tween, window, cx);
            *slot = None;
            match placement {
                DockPlacement::Left => {
                    left = dock_display_size(dock_area, placement, None, now, cx)
                }
                DockPlacement::Right => {
                    right = dock_display_size(dock_area, placement, None, now, cx)
                }
                DockPlacement::Bottom => {
                    bottom = dock_display_size(dock_area, placement, None, now, cx);
                }
                DockPlacement::Center => {}
            }
        }
    }

    (left, right, bottom, needs_frame)
}

pub fn dock_display_size(
    dock_area: &Entity<DockArea>,
    placement: DockPlacement,
    tween: Option<DockSizeTween>,
    now: Instant,
    cx: &App,
) -> f32 {
    let Some(tween) = tween else {
        let dock = dock_area.read(cx);
        return dock_entity(dock, placement)
            .map(|entity| entity.read(cx))
            .map(|dock| {
                if dock.is_open() || dock_clip_size(dock) > f32::EPSILON {
                    dock_clip_size(dock)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
    };
    eval_dock_tween(Some(tween), tween.to, now)
}

pub fn start_dock_toggle_tween<T: gpui::Render>(
    dock_area: &Entity<DockArea>,
    tweens: &mut DockTweenState,
    placement: DockPlacement,
    now: Instant,
    window: &mut Window,
    cx: &mut Context<T>,
) {
    let dock = dock_area.read(cx);
    let is_open = dock.is_dock_open(placement, cx);
    let will_open = dock_toggle_will_open(tweens, placement, is_open, now);
    let from = dock_display_size(dock_area, placement, tweens.tween(placement), now, cx);
    let rest = dock_entity(dock, placement)
        .map(|entity| entity.read(cx))
        .map(dock_rest_size)
        .unwrap_or(0.0);
    let to = if will_open { rest } else { 0.0 };

    if will_open && !is_open {
        let dock_entity = match placement {
            DockPlacement::Left => dock.left_dock().cloned(),
            DockPlacement::Right => dock.right_dock().cloned(),
            DockPlacement::Bottom => dock.bottom_dock().cloned(),
            DockPlacement::Center => None,
        };
        if let Some(dock_entity) = dock_entity {
            dock_entity.update(cx, |dock, cx| {
                dock.set_open(true, window, cx);
                dock.set_animated_size(Some(px(0.0)), cx);
            });
        }
    }

    *tweens.tween_mut(placement) = Some(DockSizeTween::new(from, to, now));
    cx.notify();
}

fn dock_toggle_will_open(
    tweens: &DockTweenState,
    placement: DockPlacement,
    is_open: bool,
    now: Instant,
) -> bool {
    if let Some(tween) = tweens.tween(placement)
        && tween.is_active(now)
    {
        return tween.to <= f32::EPSILON;
    }
    !is_open
}

pub fn layout_with_animated_docks(
    layout: ShellLayout,
    left: f32,
    right: f32,
    bottom: f32,
) -> ShellLayout {
    layout.with_animated_docks(left, right, bottom)
}
