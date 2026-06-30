//! Fullscreen chat surface for the shell.

use std::sync::Arc;

use futures::StreamExt;
use futures::channel::mpsc;

use crate::api::spawn_http_task;
use gpui::{
    App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, ScrollAnchor, ScrollHandle, StatefulInteractiveElement,
    Styled, WeakEntity, Window, div, prelude::FluentBuilder, px, relative,
};
use gpui_component::Disableable;
use gpui_component::StyledExt;
use gpui_component::IconName;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_component::h_flex;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::v_flex;

use crate::api::{
    ApiError, CancelToken, ChatMessage, ChatProvider, ChatRequest, CredentialStatus,
    CredentialStore, DEFAULT_MODEL, MessageRole, StreamEvent,
};
use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, LegacyTypeRole, OpenCoreTheme,
};

use super::chat_state::{ChatState, UiMessage};
use super::chat_store::ChatStore;
use super::credential_dialog::{self, CredentialDialogContext};
use super::credential_ui::CredentialUiState;
use super::credentials_banner;

/// In-memory assistant row before the first streamed token is persisted.
const PENDING_ASSISTANT_ID: i64 = -1;

/// GPUI entity for the single-thread chat experience.
pub struct ChatView {
    provider: Arc<dyn ChatProvider>,
    store: Arc<dyn ChatStore>,
    credentials: Arc<dyn CredentialStore>,
    state: ChatState,
    input: Entity<InputState>,
    api_key_input: Option<Entity<InputState>>,
    focus_handle: FocusHandle,
    theme: OpenCoreTheme,
    pending_clear_input: bool,
    message_scroll: ScrollHandle,
    scroll_anchor: ScrollAnchor,
    pending_scroll_to_bottom: bool,
    scroll_settle_frames: u8,
    credential_ui: CredentialUiState,
    active_stream_cancel: Option<CancelToken>,
    streaming_assistant_id: Option<i64>,
}

impl ChatView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        provider: Arc<dyn ChatProvider>,
        store: Arc<dyn ChatStore>,
        credentials: Arc<dyn CredentialStore>,
        theme: OpenCoreTheme,
    ) -> Self {
        let mut state = ChatState::default();
        match store.ensure_thread() {
            Ok(thread_id) => {
                state.thread_id = Some(thread_id);
                match store.load_messages(thread_id) {
                    Ok(messages) => {
                        state.messages = messages
                            .into_iter()
                            .map(|message| UiMessage {
                                id: message.id,
                                role: message.role,
                                content: message.content,
                            })
                            .collect();
                    }
                    Err(error) => state.set_error(error.to_string()),
                }
            }
            Err(error) => state.set_error(error.to_string()),
        }

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .placeholder("Message OpenRouter…")
        });

        let view = cx.entity().downgrade();
        cx.subscribe(
            &input,
            move |this: &mut Self, input, event: &InputEvent, cx| {
                if let InputEvent::PressEnter { shift, .. } = event
                    && !shift
                {
                    this.try_send_message(input, view.clone(), cx);
                }
            },
        )
        .detach();

        let message_scroll = ScrollHandle::default();
        let scroll_anchor = ScrollAnchor::for_handle(message_scroll.clone());
        let pending_scroll_to_bottom = !state.messages.is_empty();
        let credential_ui = CredentialUiState {
            missing: matches!(provider.credential_status(), CredentialStatus::Missing),
            banner_dismissed: false,
        };

        Self {
            provider,
            store,
            credentials,
            state,
            input,
            api_key_input: None,
            focus_handle: cx.focus_handle(),
            theme,
            pending_clear_input: false,
            message_scroll,
            scroll_anchor,
            pending_scroll_to_bottom,
            scroll_settle_frames: if pending_scroll_to_bottom { 4 } else { 0 },
            credential_ui,
            active_stream_cancel: None,
            streaming_assistant_id: None,
        }
    }

    fn refresh_credential_cache(&mut self) {
        let was_missing = self.credential_ui.missing;
        let now_missing = matches!(self.provider.credential_status(), CredentialStatus::Missing);
        self.credential_ui.refresh(was_missing, now_missing);
    }

    pub(crate) fn on_credentials_changed(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        clear_input: bool,
    ) {
        self.state.error = None;
        self.refresh_credential_cache();
        if clear_input {
            if let Some(input) = &self.api_key_input {
                input.update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
    }

    pub(crate) fn set_credential_error(&mut self, message: String) {
        self.state.set_error(message);
    }

    fn dismiss_credentials_banner(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.credential_ui.dismiss_banner();
        cx.notify();
    }

    fn cancel_active_stream(&mut self) {
        if let Some(token) = self.active_stream_cancel.take() {
            token.cancel();
        }
        self.streaming_assistant_id = None;
    }

    fn persist_assistant_content(
        &mut self,
        assistant_id: i64,
        content: &str,
        store: &Arc<dyn ChatStore>,
    ) {
        if assistant_id == PENDING_ASSISTANT_ID {
            return;
        }
        if let Err(error) = store.update_message_content(assistant_id, content) {
            self.state
                .set_error(format!("Could not save message: {error}"));
        }
    }

    fn insert_pending_assistant(
        &mut self,
        thread_id: i64,
        content: &str,
        store: &Arc<dyn ChatStore>,
    ) -> i64 {
        match store.insert_message(thread_id, MessageRole::Assistant, content) {
            Ok(id) => {
                if let Some(message) = self
                    .state
                    .messages
                    .iter_mut()
                    .find(|message| message.id == PENDING_ASSISTANT_ID)
                {
                    message.id = id;
                }
                self.streaming_assistant_id = Some(id);
                id
            }
            Err(error) => {
                self.state.set_error(error.to_string());
                PENDING_ASSISTANT_ID
            }
        }
    }

    fn mark_scroll_to_latest(&mut self) {
        self.pending_scroll_to_bottom = true;
        self.scroll_settle_frames = 4;
    }

    fn schedule_scroll_to_latest(&self, window: &mut Window) {
        self.message_scroll.scroll_to_bottom();
        let scroll = self.message_scroll.clone();
        let anchor = self.scroll_anchor.clone();
        window.on_next_frame(move |window, cx| {
            scroll.scroll_to_bottom();
            anchor.scroll_to(window, cx);
        });
    }

    fn open_credential_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.api_key_input.is_none() {
            self.api_key_input = Some(cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("sk-or-v1-…")
                    .masked(true)
            }));
        }

        let api_key_input = self
            .api_key_input
            .clone()
            .expect("api key input should exist");

        credential_dialog::open(
            window,
            cx,
            CredentialDialogContext {
                api_key_input,
                credentials: self.credentials.clone(),
                view: cx.entity().downgrade(),
            },
        );
    }

    pub fn set_theme(&mut self, theme: OpenCoreTheme) {
        self.theme = theme;
    }

    fn on_send_clicked(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.clone();
        let view = cx.entity().downgrade();
        self.try_send_message(input, view, cx);
    }

    fn try_send_message(
        &mut self,
        input: Entity<InputState>,
        view: WeakEntity<Self>,
        cx: &mut Context<Self>,
    ) {
        if !self.state.can_send(self.credential_ui.missing) {
            return;
        }

        let content = input.read(cx).value().trim().to_string();
        if content.is_empty() {
            return;
        }

        let thread_id = match self.state.thread_id {
            Some(thread_id) => thread_id,
            None => match self.store.ensure_thread() {
                Ok(thread_id) => {
                    self.state.thread_id = Some(thread_id);
                    thread_id
                }
                Err(error) => {
                    self.state.set_error(error.to_string());
                    cx.notify();
                    return;
                }
            },
        };

        let user_id = match self
            .store
            .insert_message(thread_id, MessageRole::User, &content)
        {
            Ok(id) => id,
            Err(error) => {
                self.state.set_error(error.to_string());
                cx.notify();
                return;
            }
        };

        self.state.append_user_message(user_id, content);
        self.pending_clear_input = true;
        self.mark_scroll_to_latest();

        let request_messages = self
            .state
            .messages
            .iter()
            .map(|message| ChatMessage {
                role: message.role,
                content: message.content.clone(),
            })
            .collect();

        self.cancel_active_stream();
        self.state.begin_assistant_message(PENDING_ASSISTANT_ID);
        self.streaming_assistant_id = Some(PENDING_ASSISTANT_ID);
        self.mark_scroll_to_latest();
        cx.notify();

        let provider = self.provider.clone();
        let store = self.store.clone();

        let cancel = CancelToken::new();
        self.active_stream_cancel = Some(cancel.clone());
        let (event_tx, mut event_rx) = mpsc::unbounded();
        let request = ChatRequest {
            model: DEFAULT_MODEL.to_string(),
            messages: request_messages,
        };

        spawn_http_task({
            let provider = provider.clone();
            let cancel = cancel.clone();
            async move {
                let mut stream = provider.stream_chat(request, cancel);
                while let Some(event) = stream.next().await {
                    if event_tx.unbounded_send(event).is_err() {
                        break;
                    }
                }
            }
        });

        cx.spawn(async move |_, cx| {
            while let Some(event) = event_rx.next().await {
                let update = match event {
                    Ok(StreamEvent::Token(token)) => StreamUpdate::Token(token),
                    Ok(StreamEvent::Done) => StreamUpdate::Done,
                    Err(error) => StreamUpdate::Error(format_provider_error(error)),
                };

                let should_stop = matches!(update, StreamUpdate::Done | StreamUpdate::Error(_));
                let _ = view.update(cx, |chat, cx| {
                    chat.apply_stream_update(&store, update);
                    cx.notify();
                });

                if should_stop {
                    break;
                }
            }
            let _ = view.update(cx, |chat, _| {
                chat.active_stream_cancel = None;
                if chat.state.is_streaming {
                    chat.state.finish_streaming();
                }
                chat.streaming_assistant_id = None;
            });
        })
        .detach();
    }

    fn apply_stream_update(&mut self, store: &Arc<dyn ChatStore>, update: StreamUpdate) {
        let assistant_id = match self.streaming_assistant_id {
            Some(id) => id,
            None => return,
        };
        let thread_id = match self.state.thread_id {
            Some(thread_id) => thread_id,
            None => return,
        };

        match update {
            StreamUpdate::Token(token) => {
                self.state.append_assistant_token(assistant_id, &token);
                if assistant_id == PENDING_ASSISTANT_ID {
                    let content = self
                        .state
                        .messages
                        .iter()
                        .find(|message| message.id == PENDING_ASSISTANT_ID)
                        .map(|message| message.content.clone())
                        .unwrap_or_default();
                    let _ = self.insert_pending_assistant(thread_id, &content, store);
                }
                self.mark_scroll_to_latest();
            }
            StreamUpdate::Done => {
                self.state.finish_streaming();
                if let Some(message) = self
                    .state
                    .messages
                    .iter()
                    .find(|message| message.id == assistant_id)
                {
                    let content = message.content.clone();
                    if assistant_id == PENDING_ASSISTANT_ID {
                        if content.is_empty() {
                            self.state.messages.pop();
                        } else {
                            let _ = self.insert_pending_assistant(thread_id, &content, store);
                        }
                    } else {
                        self.persist_assistant_content(assistant_id, &content, store);
                    }
                }
                self.streaming_assistant_id = None;
                self.mark_scroll_to_latest();
            }
            StreamUpdate::Error(message) => {
                if let Some(last) = self.state.messages.last()
                    && last.role == MessageRole::Assistant
                    && last.content.is_empty()
                {
                    if last.id != PENDING_ASSISTANT_ID {
                        let id = last.id;
                        if let Err(error) = store.delete_message(id) {
                            eprintln!(
                                "opencore: failed to remove empty assistant message: {error}"
                            );
                        }
                    }
                    self.state.messages.pop();
                }
                self.streaming_assistant_id = None;
                self.state.set_error(message);
            }
        }
    }
}

enum StreamUpdate {
    Token(String),
    Done,
    Error(String),
}

fn format_provider_error(error: ApiError) -> String {
    match error {
        ApiError::MissingCredentials => {
            "OpenRouter credentials are missing. Add an API key to continue.".into()
        }
        ApiError::RequestFailed(message) => format!("Request failed: {message}"),
        ApiError::ParseError(message) => format!("Could not read provider response: {message}"),
    }
}

impl Focusable for ChatView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_clear_input {
            self.pending_clear_input = false;
            self.input.update(cx, |input, cx| {
                input.set_value("", window, cx);
            });
        }

        let should_follow_latest = self.pending_scroll_to_bottom
            || self.state.is_streaming
            || self.scroll_settle_frames > 0;
        if should_follow_latest {
            self.schedule_scroll_to_latest(window);
            if self.scroll_settle_frames > 0 {
                self.scroll_settle_frames -= 1;
            }
            if !self.state.is_streaming && self.scroll_settle_frames == 0 {
                self.pending_scroll_to_bottom = false;
            }
        }

        let theme = self.theme;
        let spacing = theme.spacing;
        let inset = px(spacing.md as f32);
        let thread_bottom_pad = px(32.);
        let background = theme.surface(BackgroundToken::Primary);
        let foreground = theme.foreground(ForegroundToken::Primary);
        let muted = theme.foreground(ForegroundToken::Muted);
        let border = theme.border_token(BorderToken::Default);
        let card_bg = theme.surface(BackgroundToken::Secondary);
        let user_bubble_bg = theme.surface(BackgroundToken::Tertiary);
        let label = theme.label;
        let can_send = self.state.can_send(self.credential_ui.missing);
        let show_credentials_banner = self.credential_ui.should_show_banner();
        let error = self.state.error.clone();

        let mut content = v_flex().size_full().min_h_0().bg(background);

        content = content.child(
            h_flex()
                .flex_shrink_0()
                .w_full()
                .px(inset)
                .pt(inset)
                .pb(px(8.))
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(label.size_px as f32))
                        .font_semibold()
                        .text_color(foreground)
                        .child("Chat"),
                )
                .child(
                    Button::new("open-credential-settings")
                        .icon(IconName::Settings)
                        .ghost()
                        .small()
                        .tooltip("OpenRouter credentials")
                        .on_click(cx.listener(Self::open_credential_settings)),
                ),
        );

        if let Some(text) = error {
            content = content.child(div().flex_shrink_0().px(inset).pt(inset).child(error_panel(
                &text,
                border,
                theme.foreground(ForegroundToken::Accent),
                label,
            )));
        }

        let scroll_anchor = self.scroll_anchor.clone();

        let mut thread = v_flex().w_full().gap(px(spacing.md as f32));

        for message in &self.state.messages {
            thread = thread.child(message_row(
                message,
                foreground,
                muted,
                user_bubble_bg,
                label,
            ));
        }

        thread = thread.child(div().h(thread_bottom_pad).w_full()).child(
            div()
                .id("chat-scroll-bottom")
                .h(px(1.))
                .w_full()
                .anchor_scroll(Some(scroll_anchor)),
        );

        let message_list = v_flex()
            .w_full()
            .px(inset)
            .pt(inset)
            .pb(inset)
            .child(thread);

        content = content.child(
            div()
                .id("chat-messages-scroll")
                .flex_1()
                .min_h_0()
                .w_full()
                .track_scroll(&self.message_scroll)
                .overflow_y_scroll()
                .child(message_list)
                .vertical_scrollbar(&self.message_scroll),
        );

        let input = Input::new(&self.input)
            .h(px(72.))
            .appearance(false)
            .disabled(!can_send);

        let composer = div().flex_shrink_0().w_full().px(inset).pb(inset).child(
            v_flex()
                .w_full()
                .gap_2()
                .when(show_credentials_banner, |this| {
                    this.child(credentials_banner::credentials_banner(
                        border,
                        card_bg,
                        theme.foreground(ForegroundToken::Accent),
                        muted,
                        label,
                        cx.listener(Self::open_credential_settings),
                        cx.listener(Self::dismiss_credentials_banner),
                    ))
                })
                .child(
                    v_flex()
                        .w_full()
                        .rounded_lg()
                        .border_1()
                        .border_color(border)
                        .bg(card_bg)
                        .child(
                            div()
                                .px(px(12.))
                                .pt(px(12.))
                                .when(!can_send, |this| this.opacity(0.6))
                                .child(input),
                        )
                        .child(
                            h_flex()
                                .px(px(12.))
                                .pb(px(12.))
                                .pt(px(4.))
                                .gap(px(8.))
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(muted)
                                        .child(DEFAULT_MODEL),
                                )
                                .child(
                                    Button::new("send-message")
                                        .icon(IconName::ArrowUp)
                                        .primary()
                                        .small()
                                        .rounded(ButtonRounded::Size(px(12.)))
                                        .disabled(!can_send)
                                        .on_click(cx.listener(Self::on_send_clicked)),
                                ),
                        ),
                ),
        );

        content.child(composer)
    }
}

fn error_panel(
    text: &str,
    border: gpui::Hsla,
    foreground: gpui::Hsla,
    label: LegacyTypeRole,
) -> impl IntoElement {
    div()
        .w_full()
        .px(px(12.))
        .py(px(10.))
        .rounded_md()
        .border_1()
        .border_color(border)
        .text_size(px(label.size_px as f32))
        .text_color(foreground)
        .child(text.to_string())
}

fn message_row(
    message: &UiMessage,
    foreground: gpui::Hsla,
    muted: gpui::Hsla,
    user_bubble_bg: gpui::Hsla,
    label: LegacyTypeRole,
) -> impl IntoElement {
    let text_size = px(label.size_px as f32);
    let body = div()
        .text_size(text_size)
        .text_color(foreground)
        .child(message.content.clone());

    match message.role {
        MessageRole::User => div().w_full().flex().justify_end().child(
            div()
                .max_w(relative(0.82))
                .px(px(14.))
                .py(px(10.))
                .rounded_lg()
                .bg(user_bubble_bg)
                .child(body),
        ),
        MessageRole::Assistant => div().w_full().max_w(relative(0.92)).py(px(4.)).child(body),
        MessageRole::System => div().w_full().flex().justify_center().py(px(8.)).child(
            div()
                .text_size(px(11.))
                .text_color(muted)
                .child(message.content.clone()),
        ),
    }
}
