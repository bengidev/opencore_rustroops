//! Per-conversation custom instructions dialog.

use gpui::px;
use gpui::{App, Context, Entity, ParentElement, Styled, WeakEntity, Window};
use gpui_component::WindowExt;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::h_flex;
use gpui_component::input::{Input, InputState};
use gpui_component::v_flex;

use super::chat_view::ChatView;

pub(crate) struct InstructionsDialogContext {
    pub input: Entity<InputState>,
    pub view: WeakEntity<ChatView>,
}

pub(crate) fn open(
    window: &mut Window,
    cx: &mut Context<ChatView>,
    context: InstructionsDialogContext,
) {
    let InstructionsDialogContext { input, view } = context;

    let save = make_save_handler(input.clone(), view);

    window.open_dialog(cx, move |dialog, _window, _cx| {
        dialog
            .title("Custom Instructions")
            .child(
                v_flex()
                    .gap_2()
                    .min_w(px(320.0))
                    .child(
                        "Instructions are prepended as a system prompt for this conversation.",
                    )
                    .child(Input::new(&input).min_h(px(120.0))),
            )
            .footer(
                h_flex()
                    .w_full()
                    .gap_2()
                    .justify_between()
                    .child(
                        Button::new("cancel-instructions")
                            .label("Cancel")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("save-instructions")
                                    .label("Save")
                                    .primary()
                                    .on_click({
                                        let save = save.clone();
                                        move |_, window, cx| {
                                            save(window, cx);
                                        }
                                    }),
                            ),
                    ),
            )
    });
}

fn make_save_handler(
    input: Entity<InputState>,
    view: WeakEntity<ChatView>,
) -> impl Fn(&mut Window, &mut App) + Clone {
    move |window, cx| {
        let value = input.read(cx).value().to_string();
        let _ = view.update(cx, |chat, cx| {
            chat.set_custom_instructions(value, cx);
        });
        window.close_dialog(cx);
    }
}
