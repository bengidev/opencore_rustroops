//! OpenRouter API key settings dialog.

use std::sync::Arc;

use gpui::{App, Context, Entity, ParentElement, Styled, WeakEntity, Window};
use gpui_component::WindowExt;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::h_flex;
use gpui_component::input::{Input, InputState};
use gpui_component::v_flex;

use crate::api::CredentialStore;

use super::chat_view::ChatView;

pub(crate) struct CredentialDialogContext {
    pub api_key_input: Entity<InputState>,
    pub credentials: Arc<dyn CredentialStore>,
    pub view: WeakEntity<ChatView>,
}

pub(crate) fn open(
    window: &mut Window,
    cx: &mut Context<ChatView>,
    context: CredentialDialogContext,
) {
    let CredentialDialogContext {
        api_key_input,
        credentials,
        view,
    } = context;

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let save = make_save_handler(api_key_input.clone(), credentials.clone(), view.clone());

        dialog
            .title("OpenRouter API Key")
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        "Paste your API key from openrouter.ai. Environment variables override a saved key.",
                    )
                    .child(Input::new(&api_key_input).mask_toggle()),
            )
            .on_ok({
                let save = save.clone();
                move |_, window, cx| save(window, cx, true)
            })
            .footer(
                h_flex()
                    .w_full()
                    .gap_2()
                    .justify_between()
                    .child(
                        Button::new("clear-saved-key")
                            .label("Clear")
                            .ghost()
                            .on_click({
                                let credentials = credentials.clone();
                                let view = view.clone();
                                move |_, window, cx| {
                                    if let Err(error) = credentials.clear_api_key() {
                                        let _ = view.update(cx, |chat, cx| {
                                            chat.set_credential_error(error.to_string());
                                            cx.notify();
                                        });
                                        return;
                                    }

                                    let _ = view.update(cx, |chat, cx| {
                                        chat.on_credentials_changed(window, cx, true);
                                        window.close_dialog(cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("cancel-api-key")
                                    .label("Cancel")
                                    .on_click(move |_, window, cx| {
                                        window.close_dialog(cx);
                                    }),
                            )
                            .child(
                                Button::new("save-api-key")
                                    .label("Save")
                                    .primary()
                                    .on_click(move |_, window, cx| {
                                        let _ = save(window, cx, true);
                                    }),
                            ),
                    ),
            )
    });
}

fn make_save_handler(
    api_key_input: Entity<InputState>,
    credentials: Arc<dyn CredentialStore>,
    view: WeakEntity<ChatView>,
) -> impl Fn(&mut Window, &mut App, bool) -> bool + Clone {
    move |window, cx, close_on_success| {
        let key = api_key_input.read(cx).value().trim().to_string();
        if key.is_empty() {
            return false;
        }

        if let Err(error) = credentials.save_api_key(&key) {
            let _ = view.update(cx, |chat, cx| {
                chat.set_credential_error(error.to_string());
                cx.notify();
            });
            return false;
        }

        let _ = view.update(cx, |chat, cx| {
            chat.on_credentials_changed(window, cx, true);
            if close_on_success {
                window.close_dialog(cx);
            }
            cx.notify();
        });
        true
    }
}
