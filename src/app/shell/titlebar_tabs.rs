//! Center workspace tab strip for the window title bar (Task 4).

use gpui::{App, Entity, IntoElement, ParentElement, Styled, px};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable, TitleBar,
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
};

use super::panels::CenterStubHost;

/// Renders the center tab strip for [`CenterStubHost`], suitable as a [`TitleBar`] child.
pub fn render_center_tab_bar(host: &Entity<CenterStubHost>, cx: &App) -> impl IntoElement {
    let host_state = host.read(cx);
    let active_ix = host_state.active_ix();
    let titles = host_state.tab_titles(cx);
    let can_close = host_state.tab_count() > 1;

    let host_select = host.clone();
    let host_add = host.clone();

    let mut tab_bar = TabBar::new("center-tabs")
        .mt(px(1.))
        .segmented()
        .px_0()
        .py(px(2.))
        .bg(cx.theme().title_bar)
        .selected_index(active_ix)
        .on_click(move |ix, _, cx| {
            host_select.update(cx, |host, cx| {
                host.select(*ix);
                cx.notify();
            });
        });

    for (ix, title) in titles.into_iter().enumerate() {
        let mut tab = Tab::new().label(title);
        if can_close {
            let host_close = host.clone();
            tab = tab.suffix(
                Button::new(format!("center-tab-close-{ix}"))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        host_close.update(cx, |host, cx| {
                            host.close_tab(ix);
                            cx.notify();
                        });
                    }),
            );
        }
        tab_bar = tab_bar.child(tab);
    }

    tab_bar.suffix(
        Button::new("center-tab-add")
            .ghost()
            .xsmall()
            .icon(IconName::Plus)
            .on_click(move |_, _, cx| {
                host_add.update(cx, |host, cx| {
                    host.add_tab(cx);
                    cx.notify();
                });
            }),
    )
}

/// Title bar with the center tab strip pre-wired for [`CenterStubHost`].
pub fn center_title_bar(host: &Entity<CenterStubHost>, cx: &App) -> TitleBar {
    TitleBar::new().child(render_center_tab_bar(host, cx))
}
