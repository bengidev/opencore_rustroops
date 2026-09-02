//! Atom row surfaces: card layout for inbox/pinned, slim layout for history.

use gpui::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder as _, px, relative,
};
use gpui_component::{
    Icon, IconName, Sizable, h_flex,
    input::{Input, InputState},
    menu::{ContextMenuExt as _, PopupMenuItem},
    v_flex,
};

use crate::shared::theme::{BackgroundToken, ForegroundToken, OpenCoreTheme, TypeRole};

use super::super::demo_data::{DEMO_PROJECTS, DemoAtom, AtomShelf, AtomStatus};
use super::super::state::SidebarViewModel;
use super::super::surfaces::{
    drop_line_color, pr_open_color, project_favicon_color, row_active_bg, row_hover_bg,
    row_selected_bg, status_color,
};
use super::super::tokens::{
    FAVICON_SIZE, ROW_CONTENT_INSET, ROW_HEIGHT_CARD, ROW_HEIGHT_SLIM, ROW_RADIUS,
};
use super::callbacks::{
    AtomDragOverCallback, AtomDropCallback, AtomHoverCallback, AtomIdCallback,
    AtomMoveCallback, AtomSelectCallback,
};
use super::pinned_drag::{PinnedRowDragUi, PinnedAtomDrag, AtomDragScope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomRowVariant {
    Card,
    Slim,
}

#[derive(Clone)]
pub struct AtomRowActions {
    pub on_activate: AtomIdCallback,
    pub on_select: AtomSelectCallback,
    pub on_hover: AtomHoverCallback,
    pub on_move_atom: AtomMoveCallback,
    pub on_pin: AtomIdCallback,
    pub on_unpin: AtomIdCallback,
    pub on_settle: AtomIdCallback,
    pub on_unsettle: AtomIdCallback,
    pub on_rename: AtomIdCallback,
    pub on_archive: AtomIdCallback,
    pub on_unarchive: AtomIdCallback,
    pub on_pinned_drag_start: AtomIdCallback,
    pub on_pinned_drag_over: AtomDragOverCallback,
    pub on_pinned_drop: AtomDropCallback,
}

pub fn sidebar_atom_row(
    atom: &DemoAtom,
    variant: AtomRowVariant,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &AtomRowActions,
    rename_input: Option<&Entity<InputState>>,
    pinned_drag: Option<PinnedRowDragUi>,
    scroll_anchor: Option<gpui::ScrollAnchor>,
) -> AnyElement {
    match variant {
        AtomRowVariant::Card => card_row(
            atom,
            view,
            theme,
            actions,
            rename_input,
            pinned_drag,
            scroll_anchor,
        )
        .into_any_element(),
        AtomRowVariant::Slim => slim_row(
            atom,
            view,
            theme,
            actions,
            rename_input,
            pinned_drag,
            scroll_anchor,
        )
        .into_any_element(),
    }
}

pub fn sidebar_show_more_button(
    theme: &OpenCoreTheme,
    on_show_more: impl Fn(&mut gpui::Window, &mut App) + 'static,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    div()
        .id("left-sidebar-show-more-settled")
        .w_full()
        .py(px(8.))
        .text_center()
        .cursor_pointer()
        .font_family(mono_family())
        .text_size(px(TypeRole::LabelMd.size()))
        .text_color(muted)
        .hover(|style| style.text_color(theme.foreground(ForegroundToken::Primary)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            on_show_more(window, cx)
        })
        .child("Show more")
}

fn row_surface(
    atom: &DemoAtom,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    height: f32,
    variant_key: &str,
    actions: &AtomRowActions,
    pinned_drag: Option<PinnedRowDragUi>,
    scroll_anchor: Option<gpui::ScrollAnchor>,
    content: impl IntoElement,
) -> impl IntoElement {
    let is_active = view.is_active(atom);
    let is_selected = view.is_selected(atom);
    let recede = view.should_recede(atom);

    let (bg, text) = if is_active {
        (
            row_active_bg(theme),
            theme.foreground(ForegroundToken::Primary),
        )
    } else if is_selected {
        (
            row_selected_bg(theme),
            theme.foreground(ForegroundToken::Primary),
        )
    } else {
        (
            theme.surface(BackgroundToken::Primary),
            if recede {
                theme.foreground(ForegroundToken::Muted).alpha(0.75)
            } else {
                theme.foreground(ForegroundToken::Primary)
            },
        )
    };

    let atom_id = atom.id.to_string();
    let on_select = actions.on_select.clone();
    let on_activate = actions.on_activate.clone();
    let on_hover = actions.on_hover.clone();
    let on_pinned_drag_start = actions.on_pinned_drag_start.clone();
    let on_pinned_drag_over = actions.on_pinned_drag_over.clone();
    let on_pinned_drop = actions.on_pinned_drop.clone();
    let is_pinned = view.effective_shelf(atom) == AtomShelf::Pinned;
    let is_settled = view.effective_shelf(atom) == AtomShelf::Settled;
    let is_archived = view.is_archived(atom);
    let drag_scope =
        AtomDragScope::from_atom(atom.id, view.effective_shelf(atom), is_archived);
    let menu_title = view.display_title(atom);
    let row_drag = pinned_drag.unwrap_or_default();
    let drag_hover_bg = row_hover_bg(theme);

    let is_card = variant_key == "card";

    div()
        .id(format!("left-sidebar-atom-{variant_key}-{}", atom.id))
        .anchor_scroll(scroll_anchor)
        .w_full()
        .min_w_0()
        .relative()
        .flex()
        .when(is_card, |row| {
            row.h(px(height)).items_start().overflow_x_hidden()
        })
        .when(!is_card, |row| {
            row.h(px(height)).items_center().overflow_hidden()
        })
        .bg(bg)
        .text_color(text)
        .cursor_pointer()
        .when(row_drag.drop_above, |row| {
            row.child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(0.))
                    .right(px(0.))
                    .h(px(2.))
                    .bg(drop_line_color(theme)),
            )
        })
        .when(row_drag.drop_below, |row| {
            row.child(
                div()
                    .absolute()
                    .bottom(px(0.))
                    .left(px(0.))
                    .right(px(0.))
                    .h(px(2.))
                    .bg(drop_line_color(theme)),
            )
        })
        .when(!is_active && !is_selected, |row| {
            row.hover(|style| style.bg(row_hover_bg(theme)))
        })
        .when(recede && !is_active && !is_selected, |row| {
            row.opacity(0.85)
        })
        .when(row_drag.is_source, |row| row.opacity(0.45))
        .on_drag(
            PinnedAtomDrag {
                atom_id: atom_id.clone(),
                scope: drag_scope,
                title: menu_title.clone().into(),
                preview_bg: theme.surface(BackgroundToken::Secondary),
                preview_text: theme.foreground(ForegroundToken::Primary),
            },
            {
                let on_pinned_drag_start = on_pinned_drag_start.clone();
                move |drag, _, window, cx| {
                    cx.stop_propagation();
                    on_pinned_drag_start(drag.atom_id.clone(), window, cx);
                    cx.new(|_| PinnedDragPreview {
                        title: drag.title.clone(),
                        bg: drag.preview_bg,
                        text: drag.preview_text,
                    })
                }
            },
        )
        .can_drop({
            let self_id = atom_id.clone();
            let self_scope = drag_scope;
            move |value, _, _| {
                value
                    .downcast_ref::<PinnedAtomDrag>()
                    .is_some_and(|drag| {
                        drag.atom_id != self_id && drag.scope.allows_drop(self_scope)
                    })
            }
        })
        .drag_over::<PinnedAtomDrag>({
            let self_id = atom_id.clone();
            let self_scope = drag_scope;
            move |style, drag, _, _| {
                if drag.atom_id == self_id || !drag.scope.allows_drop(self_scope) {
                    return style;
                }
                style.bg(drag_hover_bg)
            }
        })
        .on_drag_move::<PinnedAtomDrag>({
            let target_id = atom_id.clone();
            let self_scope = drag_scope;
            let on_pinned_drag_over = on_pinned_drag_over.clone();
            move |event, window, cx| {
                let drag = event.drag(cx);
                if drag.atom_id == target_id || !drag.scope.allows_drop(self_scope) {
                    return;
                }
                let pointer = event.event.position;
                let bounds = event.bounds;
                if pointer.x < bounds.origin.x
                    || pointer.x > bounds.origin.x + bounds.size.width
                    || pointer.y < bounds.origin.y
                    || pointer.y > bounds.origin.y + bounds.size.height
                {
                    return;
                }
                let midpoint = bounds.size.height / 2.0;
                let offset_y = pointer.y - bounds.origin.y;
                let insert_after = offset_y > midpoint;
                on_pinned_drag_over(target_id.clone(), insert_after, window, cx);
            }
        })
        .on_drop({
            let target_id = atom_id.clone();
            let self_scope = drag_scope;
            let on_pinned_drop = on_pinned_drop.clone();
            move |drag: &PinnedAtomDrag, window, cx| {
                if drag.atom_id == target_id || !drag.scope.allows_drop(self_scope) {
                    return;
                }
                on_pinned_drop(drag.atom_id.clone(), target_id.clone(), window, cx);
            }
        })
        .on_hover({
            let on_hover = on_hover.clone();
            let hover_id = atom_id.clone();
            move |hovered, window, cx| {
                on_hover(
                    if *hovered {
                        Some(hover_id.clone())
                    } else {
                        None
                    },
                    window,
                    cx,
                );
            }
        })
        .on_click({
            let on_select = on_select.clone();
            let on_activate = on_activate.clone();
            let click_id = atom_id.clone();
            move |event: &ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    return;
                }
                if event.modifiers().shift {
                    on_select(click_id.clone(), true, window, cx);
                } else {
                    on_activate(click_id.clone(), window, cx);
                }
            }
        })
        .context_menu({
            let actions = actions.clone();
            let menu_id = atom_id.clone();
            let menu_title = menu_title.clone();
            let menu_state = AtomContextMenuState {
                is_pinned,
                is_settled,
                is_archived,
                can_move_up: view.can_move_atom(&atom_id, -1),
                can_move_down: view.can_move_atom(&atom_id, 1),
            };
            move |menu, window, cx| {
                build_atom_context_menu(
                    menu,
                    menu_id.clone(),
                    menu_title.clone(),
                    menu_state,
                    &actions,
                    window,
                    cx,
                )
            }
        })
        .child(content)
}

#[derive(Clone, Copy)]
struct AtomContextMenuState {
    is_pinned: bool,
    is_settled: bool,
    is_archived: bool,
    can_move_up: bool,
    can_move_down: bool,
}

fn build_atom_context_menu(
    mut menu: gpui_component::menu::PopupMenu,
    menu_id: String,
    menu_title: String,
    state: AtomContextMenuState,
    actions: &AtomRowActions,
    _window: &mut Window,
    _cx: &mut Context<gpui_component::menu::PopupMenu>,
) -> gpui_component::menu::PopupMenu {
    let AtomContextMenuState {
        is_pinned,
        is_settled,
        is_archived,
        can_move_up,
        can_move_down,
    } = state;
    menu = menu.label(menu_title);

    if can_move_up {
        let up_id = menu_id.clone();
        let on_move_up = actions.on_move_atom.clone();
        menu = menu.item(
            PopupMenuItem::new("Move up").on_click(move |_, window, cx| {
                on_move_up(up_id.clone(), -1, window, cx);
            }),
        );
    }
    if can_move_down {
        let down_id = menu_id.clone();
        let on_move_down = actions.on_move_atom.clone();
        menu = menu.item(
            PopupMenuItem::new("Move down").on_click(move |_, window, cx| {
                on_move_down(down_id.clone(), 1, window, cx);
            }),
        );
    }
    if can_move_up || can_move_down {
        menu = menu.separator();
    }

    if is_archived {
        menu = menu.item(PopupMenuItem::new("Unarchive").on_click({
            let unarchive_id = menu_id.clone();
            let on_unarchive = actions.on_unarchive.clone();
            move |_, window, cx| {
                on_unarchive(unarchive_id.clone(), window, cx);
            }
        }));
    } else if is_pinned {
        menu = menu.item(PopupMenuItem::new("Unpin").on_click({
            let unpin_id = menu_id.clone();
            let on_unpin = actions.on_unpin.clone();
            move |_, window, cx| {
                on_unpin(unpin_id.clone(), window, cx);
            }
        }));
    } else if is_settled {
        menu = menu.item(PopupMenuItem::new("Unsettle").on_click({
            let unsettle_id = menu_id.clone();
            let on_unsettle = actions.on_unsettle.clone();
            move |_, window, cx| {
                on_unsettle(unsettle_id.clone(), window, cx);
            }
        }));
    } else {
        menu = menu.item(PopupMenuItem::new("Pin").on_click({
            let pin_id = menu_id.clone();
            let on_pin = actions.on_pin.clone();
            move |_, window, cx| {
                on_pin(pin_id.clone(), window, cx);
            }
        }));
        menu = menu.item(PopupMenuItem::new("Settle").on_click({
            let settle_id = menu_id.clone();
            let on_settle = actions.on_settle.clone();
            move |_, window, cx| {
                on_settle(settle_id.clone(), window, cx);
            }
        }));
        menu = menu.item(PopupMenuItem::new("Archive").on_click({
            let archive_id = menu_id.clone();
            let on_archive = actions.on_archive.clone();
            move |_, window, cx| {
                on_archive(archive_id.clone(), window, cx);
            }
        }));
    }

    menu = menu.separator();
    menu = menu.item(PopupMenuItem::new("Rename").on_click({
        let rename_id = menu_id;
        let on_rename = actions.on_rename.clone();
        move |_, window, cx| {
            on_rename(rename_id.clone(), window, cx);
        }
    }));
    menu
}

fn card_row(
    atom: &DemoAtom,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &AtomRowActions,
    rename_input: Option<&Entity<InputState>>,
    pinned_drag: Option<PinnedRowDragUi>,
    scroll_anchor: Option<gpui::ScrollAnchor>,
) -> impl IntoElement {
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let mono = mono_family();
    let recede = view.should_recede(atom);
    let title = view.display_title(atom);
    let favicon_hue = DEMO_PROJECTS
        .iter()
        .find(|p| p.key == atom.project_key)
        .map(|p| p.favicon_hue)
        .unwrap_or(0x888888);

    row_surface(
        atom,
        view,
        theme,
        ROW_HEIGHT_CARD,
        "card",
        actions,
        pinned_drag,
        scroll_anchor,
        div()
            .w_full()
            .min_w_0()
            .px(px(ROW_CONTENT_INSET))
            .py(px(6.))
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(3.))
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .items_center()
                            .gap(px(6.))
                            .child(project_favicon(favicon_hue))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .font_family(mono.clone())
                                    .text_size(px(TypeRole::LabelMd.size()))
                                    .line_height(relative(TypeRole::LabelMd.line_height()))
                                    .text_color(if recede {
                                        secondary.alpha(0.75)
                                    } else {
                                        secondary
                                    })
                                    .child(atom.project_title),
                            )
                            .child(status_or_time(atom, view, theme, true)),
                    )
                    .child(title_row(
                        atom,
                        view,
                        &title,
                        rename_input,
                        TypeRole::LabelMd,
                        if recede {
                            theme.foreground(ForegroundToken::Secondary)
                        } else {
                            theme.foreground(ForegroundToken::Primary).alpha(0.9)
                        },
                    ))
                    .child(branch_meta_row(atom, theme)),
            ),
    )
}

fn slim_row(
    atom: &DemoAtom,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    actions: &AtomRowActions,
    rename_input: Option<&Entity<InputState>>,
    pinned_drag: Option<PinnedRowDragUi>,
    scroll_anchor: Option<gpui::ScrollAnchor>,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let recede = view.should_recede(atom);
    let title = view.display_title(atom);

    row_surface(
        atom,
        view,
        theme,
        ROW_HEIGHT_SLIM,
        "slim",
        actions,
        pinned_drag,
        scroll_anchor,
        h_flex()
            .w_full()
            .min_w_0()
            .h_full()
            .px(px(10.))
            .gap(px(10.))
            .items_center()
            .overflow_hidden()
            .child(
                Icon::new(IconName::Inbox)
                    .text_color(muted)
                    .small()
                    .flex_shrink_0(),
            )
            .child(title_row(
                atom,
                view,
                &title,
                rename_input,
                TypeRole::LabelMd,
                if recede {
                    muted.alpha(0.7)
                } else {
                    theme.foreground(ForegroundToken::Primary)
                },
            ))
            .child(status_or_time(atom, view, theme, false)),
    )
}

fn title_row(
    atom: &DemoAtom,
    view: &SidebarViewModel,
    title: &str,
    rename_input: Option<&Entity<InputState>>,
    type_role: TypeRole,
    text_color: gpui::Hsla,
) -> gpui::AnyElement {
    let mono = mono_family();
    let title = title.to_string();
    if view.is_renaming(atom)
        && let Some(rename_input) = rename_input
    {
        return div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .child(
                Input::new(rename_input)
                    .w_full()
                    .h(px(ROW_HEIGHT_SLIM - 4.))
                    .text_size(px(type_role.size()))
                    .bordered(false)
                    .appearance(false)
                    .cleanable(false),
            )
            .into_any_element();
    }

    div()
        .w_full()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .font_family(mono)
        .text_size(px(type_role.size()))
        .line_height(relative(type_role.line_height()))
        .text_color(text_color)
        .child(title)
        .into_any_element()
}

fn project_favicon(hue: u32) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .w(px(FAVICON_SIZE))
        .h(px(FAVICON_SIZE))
        .rounded(px(3.))
        .bg(project_favicon_color(hue).alpha(0.25))
        .border_1()
        .border_color(project_favicon_color(hue).alpha(0.5))
}

fn status_or_time(
    atom: &DemoAtom,
    view: &SidebarViewModel,
    theme: &OpenCoreTheme,
    show_status: bool,
) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let mono = mono_family();
    let dimmed = view.should_recede(atom) && !view.is_active(atom);

    if show_status && let Some(label) = atom.status.label() {
        let color = status_color(atom.status, theme, dimmed);
        return h_flex()
            .flex_shrink_0()
            .gap(px(4.))
            .items_center()
            .children(status_icon(atom.status, color))
            .child(
                div()
                    .font_family(mono)
                    .text_size(px(TypeRole::LabelMd.size()))
                    .text_color(color)
                    .child(label),
            )
            .into_any_element();
    }

    div()
        .flex_shrink_0()
        .font_family(mono)
        .text_size(px(TypeRole::LabelMd.size()))
        .text_color(muted)
        .child(atom.time_label)
        .into_any_element()
}

fn status_icon(status: AtomStatus, color: gpui::Hsla) -> Option<gpui::AnyElement> {
    match status {
        AtomStatus::Working => Some(
            Icon::new(IconName::LoaderCircle)
                .text_color(color)
                .small()
                .flex_shrink_0()
                .into_any_element(),
        ),
        AtomStatus::Woke => Some(
            Icon::new(IconName::Bell)
                .text_color(color)
                .small()
                .flex_shrink_0()
                .into_any_element(),
        ),
        _ => None,
    }
}

fn branch_meta_row(atom: &DemoAtom, theme: &OpenCoreTheme) -> impl IntoElement {
    let muted = theme.foreground(ForegroundToken::Muted);
    let green = pr_open_color();
    let red = theme.foreground(ForegroundToken::Accent);
    let mono = mono_family();

    h_flex()
        .w_full()
        .min_w_0()
        .items_center()
        .gap(px(6.))
        .overflow_hidden()
        .text_size(px(TypeRole::LabelMd.size()))
        .child(atom.branch.map_or_else(
            || div().flex_1().min_w_0().into_any_element(),
            |branch| {
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .font_family(mono.clone())
                    .text_color(muted)
                    .child(branch)
                    .into_any_element()
            },
        ))
        .children(terminal_indicator(atom, muted))
        .children(pr_badge(atom, muted, mono.clone()))
        .children(diff_stats(atom, green, red, mono))
}

fn terminal_indicator(atom: &DemoAtom, muted: gpui::Hsla) -> Vec<gpui::AnyElement> {
    if atom.terminal_process_count == 0 {
        return Vec::new();
    }
    vec![
        Icon::new(IconName::SquareTerminal)
            .text_color(muted)
            .small()
            .flex_shrink_0()
            .into_any_element(),
    ]
}

fn pr_badge(atom: &DemoAtom, color: gpui::Hsla, mono: SharedString) -> Vec<gpui::AnyElement> {
    atom
        .pr_number
        .map(|number| {
            div()
                .flex_shrink_0()
                .font_family(mono)
                .text_color(color)
                .child(format!("#{number}"))
                .into_any_element()
        })
        .into_iter()
        .collect()
}

fn diff_stats(
    atom: &DemoAtom,
    green: gpui::Hsla,
    red: gpui::Hsla,
    mono: SharedString,
) -> Vec<gpui::AnyElement> {
    if atom.diff_insertions.is_none() && atom.diff_deletions.is_none() {
        return Vec::new();
    }

    let mut elements = Vec::new();
    if let Some(n) = atom.diff_insertions {
        elements.push(
            div()
                .flex_shrink_0()
                .font_family(mono.clone())
                .text_color(green)
                .child(format!("+{n}"))
                .into_any_element(),
        );
    }
    if let Some(n) = atom.diff_deletions {
        elements.push(
            div()
                .flex_shrink_0()
                .font_family(mono)
                .text_color(red)
                .child(format!("−{n}"))
                .into_any_element(),
        );
    }
    elements
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}

struct PinnedDragPreview {
    title: SharedString,
    bg: gpui::Hsla,
    text: gpui::Hsla,
}

impl Render for PinnedDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(240.))
            .px(px(10.))
            .py(px(6.))
            .rounded(px(ROW_RADIUS))
            .bg(self.bg)
            .opacity(0.92)
            .shadow_md()
            .font_family(mono_family())
            .text_size(px(TypeRole::LabelMd.size()))
            .line_height(relative(TypeRole::LabelMd.line_height()))
            .text_color(self.text)
            .child(self.title.clone())
    }
}
