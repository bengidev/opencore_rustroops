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
use gpui_component::IconName;
use gpui_component::Sizable;
use gpui_component::StyledExt;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::h_flex;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::SearchableVec;
use gpui_component::select::{SelectEvent, SelectState};
use gpui_component::v_flex;

use crate::api::{
    ApiError, CancelToken, ChatProvider, ChatRequest, CredentialStatus, CredentialStore,
    DEFAULT_MODEL, MessageRole, StreamEvent, filter_generation_for_model,
};
use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, LegacyTypeRole, OpenCoreTheme, ThemeMode,
};

use super::chat_state::{ChatState, UiMessage};
use super::chat_store::ChatStore;
use super::composer_actions::ComposerActions;
use super::composer_toolbar::{ComposerToolbarProps, render_composer_toolbar};
use super::credential_dialog::{self, CredentialDialogContext};
use super::credential_ui::CredentialUiState;
use super::credentials_banner;
use super::generation_ui::model_unavailable_message;
use super::model_catalog_store::ModelCatalogStore;
use super::model_picker::{
    ModelSelectEntry, entries_from_models, persist_model_selection, selected_index_for_model,
    sync_model_select,
};
use super::stream_indicator::{
    assistant_stream_status, bounded_message_text, render_assistant_stream_body,
};

/// In-memory assistant row before the first streamed token is persisted.
const PENDING_ASSISTANT_ID: i64 = -1;

/// GPUI entity for the single-thread chat experience.
pub struct ChatView {
    provider: Arc<dyn ChatProvider>,
    store: Arc<dyn ChatStore>,
    catalog_store: Arc<dyn ModelCatalogStore>,
    credentials: Arc<dyn CredentialStore>,
    state: ChatState,
    input: Entity<InputState>,
    api_key_input: Option<Entity<InputState>>,
    model_select: Entity<SelectState<SearchableVec<ModelSelectEntry>>>,
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
    pending_model_select_sync: bool,
    pending_assistant_insert_failed: bool,
}

impl ChatView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        provider: Arc<dyn ChatProvider>,
        store: Arc<dyn ChatStore>,
        catalog_store: Arc<dyn ModelCatalogStore>,
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
                            .filter(|message| !message.content.trim().is_empty())
                            .map(|message| UiMessage {
                                id: message.id,
                                role: message.role,
                                content: message.content,
                            })
                            .collect();
                    }
                    Err(error) => state.set_error(error.to_string()),
                }
                match store.load_thread_settings(thread_id) {
                    Ok(settings) => state.thread_settings = settings,
                    Err(error) => state.set_error(error.to_string()),
                }
            }
            Err(error) => state.set_error(error.to_string()),
        }

        match catalog_store.load_catalog() {
            Ok(catalog) => {
                state.catalog.models = catalog.models;
                state.catalog.fetched_at = catalog.fetched_at;
            }
            Err(error) => state.set_error(error.to_string()),
        }

        if let Some(model) = state.catalog.model_for_id(&state.thread_settings.model_id) {
            model.sanitize_generation(&mut state.thread_settings.generation);
        }

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .placeholder("Message OpenRouter…")
        });

        let selected_model_index =
            selected_index_for_model(&state.catalog.models, &state.thread_settings.model_id);
        let model_select = cx.new(|cx| {
            SelectState::new(
                entries_from_models(&state.catalog.models),
                selected_model_index,
                window,
                cx,
            )
            .searchable(true)
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

        let view = cx.entity().downgrade();
        let store_for_model = store.clone();
        cx.subscribe(
            &model_select,
            move |this, _, event: &SelectEvent<SearchableVec<ModelSelectEntry>>, cx| {
                if let SelectEvent::Confirm(Some(model_id)) = event {
                    if let Some(model) = this.state.catalog.model_for_id(model_id) {
                        model.sanitize_generation(&mut this.state.thread_settings.generation);
                    }
                    if let Some(thread_id) = this.state.thread_id
                        && let Err(error) = persist_model_selection(
                            &store_for_model,
                            thread_id,
                            &mut this.state.thread_settings,
                            model_id.clone(),
                        )
                    {
                        this.state
                            .set_error(format!("Could not save model selection: {error}"));
                    }
                    cx.notify();
                }
                let _ = view.clone();
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

        let mut chat = Self {
            provider: provider.clone(),
            store,
            catalog_store,
            credentials,
            state,
            input,
            api_key_input: None,
            model_select,
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
            pending_model_select_sync: false,
            pending_assistant_insert_failed: false,
        };

        chat.refresh_catalog_in_background(cx);
        chat
    }

    fn persist_generation_settings(&mut self, _cx: &App) {
        let Some(thread_id) = self.state.thread_id else {
            return;
        };
        if let Err(error) = self
            .store
            .save_thread_settings(thread_id, &self.state.thread_settings)
        {
            self.state
                .set_error(format!("Could not save generation settings: {error}"));
        }
    }

    pub(crate) fn set_speed_mode(&mut self, mode: crate::api::SpeedMode, cx: &mut Context<Self>) {
        if !self.current_model_supports(|model| model.supports_speed_mode_controls()) {
            return;
        }
        self.state.thread_settings.generation.speed_mode = mode;
        self.persist_generation_settings(cx);
        cx.notify();
    }

    pub(crate) fn set_reasoning_effort(&mut self, value: &str, cx: &mut Context<Self>) {
        let Some(model) = self
            .state
            .catalog
            .model_for_id(&self.state.thread_settings.model_id)
        else {
            return;
        };
        if !model.supports_thinking_controls() {
            return;
        }
        self.state.thread_settings.generation.reasoning_effort = if value == "default" {
            None
        } else if model.is_supported_thinking_effort(value) {
            Some(value.to_string())
        } else {
            return;
        };
        self.persist_generation_settings(cx);
        cx.notify();
    }

    fn current_model_supports(
        &self,
        predicate: impl FnOnce(&crate::api::ModelInfo) -> bool,
    ) -> bool {
        self.state
            .catalog
            .model_for_id(&self.state.thread_settings.model_id)
            .is_some_and(predicate)
    }

    fn reconcile_settings_after_catalog_refresh(&mut self) {
        let previous_model_id = self.state.thread_settings.model_id.clone();
        if let Some(model) = self
            .state
            .catalog
            .model_for_id(&self.state.thread_settings.model_id)
        {
            model.sanitize_generation(&mut self.state.thread_settings.generation);
            return;
        }

        if self.state.catalog.models.is_empty() {
            return;
        }

        self.state.thread_settings.model_id = DEFAULT_MODEL.to_string();
        if let Some(model) = self
            .state
            .catalog
            .model_for_id(&self.state.thread_settings.model_id)
        {
            model.sanitize_generation(&mut self.state.thread_settings.generation);
        }
        self.state.set_error(format!(
            "{} Switched to Auto.",
            model_unavailable_message(&previous_model_id)
        ));

        if let Some(thread_id) = self.state.thread_id
            && let Err(error) = self
                .store
                .save_thread_settings(thread_id, &self.state.thread_settings)
        {
            self.state
                .set_error(format!("Could not save model selection: {error}"));
        }
    }

    fn refresh_catalog_in_background(&mut self, cx: &mut Context<Self>) {
        if self.credential_ui.missing || self.state.catalog.is_refreshing {
            return;
        }

        self.state.catalog.is_refreshing = true;
        let provider = self.provider.clone();
        let catalog_store = self.catalog_store.clone();
        let view = cx.entity().downgrade();
        let watchdog_view = view.clone();

        cx.spawn(async move |_, cx| {
            tokio::time::sleep(std::time::Duration::from_secs(90)).await;
            let _ = watchdog_view.update(cx, |chat, cx| {
                if chat.state.catalog.is_refreshing {
                    chat.state.catalog.is_refreshing = false;
                    chat.state.set_error(
                        "Model catalog refresh timed out. Try again from settings.".into(),
                    );
                    cx.notify();
                }
            });
        })
        .detach();

        cx.spawn(async move |_, cx| {
            let result = provider.list_models().await;
            let _ = view.update(cx, |chat, cx| {
                chat.state.catalog.is_refreshing = false;
                match result {
                    Ok(models) => {
                        let fetched_at = catalog_fetched_at_now();
                        if let Err(error) = catalog_store.save_catalog(&models, &fetched_at) {
                            chat.state
                                .set_error(format!("Could not cache model catalog: {error}"));
                            return;
                        }
                        chat.state.catalog.replace_catalog(models, fetched_at);
                        chat.reconcile_settings_after_catalog_refresh();
                        chat.pending_model_select_sync = true;
                    }
                    Err(error) => {
                        if chat.state.catalog.models.is_empty() {
                            chat.state
                                .set_error(format!("Could not refresh model catalog: {error}"));
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
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
        if clear_input && let Some(input) = &self.api_key_input {
            input.update(cx, |state, cx| state.set_value("", window, cx));
        }
        if !self.credential_ui.missing {
            self.refresh_catalog_in_background(cx);
        }
    }

    pub(crate) fn set_credential_error(&mut self, message: String) {
        self.state.set_error(message);
    }

    fn dismiss_credentials_banner(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.credential_ui.dismiss_banner();
        cx.notify();
    }

    fn cancel_active_stream(&mut self) {
        if let Some(token) = self.active_stream_cancel.take() {
            token.cancel();
        }
        self.state.remove_trailing_empty_assistants();
        if self.state.is_streaming {
            self.state.finish_streaming();
        }
        self.streaming_assistant_id = None;
        self.pending_assistant_insert_failed = false;
    }

    fn cleanup_inflight_assistant(&mut self, store: &Arc<dyn ChatStore>) {
        while let Some(last) = self.state.messages.last() {
            if last.role == MessageRole::Assistant && last.content.trim().is_empty() {
                let id = last.id;
                self.state.messages.pop();
                if id != PENDING_ASSISTANT_ID
                    && let Err(error) = store.delete_message(id)
                {
                    eprintln!("opencore: failed to remove empty assistant message: {error}");
                }
            } else {
                break;
            }
        }
        if self.state.is_streaming {
            self.state.finish_streaming();
        }
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

    fn open_credential_settings(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    pub(crate) fn on_send_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

        self.persist_generation_settings(cx);

        if let Err(message) = self
            .state
            .catalog
            .validate_model_id(&self.state.thread_settings.model_id)
        {
            self.state.set_error(message);
            cx.notify();
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

        let request_messages = self.state.api_messages();

        self.cancel_active_stream();
        self.pending_assistant_insert_failed = false;
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
            model: self.state.thread_settings.model_id.clone(),
            messages: request_messages,
            generation: filter_generation_for_model(
                self.state
                    .catalog
                    .model_for_id(&self.state.thread_settings.model_id),
                &self.state.thread_settings.generation,
            ),
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
                chat.cleanup_inflight_assistant(&store);
                chat.streaming_assistant_id = None;
                chat.pending_assistant_insert_failed = false;
            });
        })
        .detach();
    }

    fn apply_stream_update(&mut self, store: &Arc<dyn ChatStore>, update: StreamUpdate) {
        match update {
            StreamUpdate::Error(message) => {
                self.cleanup_inflight_assistant(store);
                self.streaming_assistant_id = None;
                self.state.set_error(message);
            }
            StreamUpdate::Token(token) => {
                let assistant_id = match self.streaming_assistant_id {
                    Some(id) => id,
                    None => return,
                };
                let thread_id = match self.state.thread_id {
                    Some(thread_id) => thread_id,
                    None => return,
                };
                self.state.append_assistant_token(assistant_id, &token);
                if assistant_id == PENDING_ASSISTANT_ID && !self.pending_assistant_insert_failed {
                    let content = self
                        .state
                        .messages
                        .iter()
                        .find(|message| message.id == PENDING_ASSISTANT_ID)
                        .map(|message| message.content.clone())
                        .unwrap_or_default();
                    let inserted_id = self.insert_pending_assistant(thread_id, &content, store);
                    if inserted_id == PENDING_ASSISTANT_ID {
                        self.pending_assistant_insert_failed = true;
                    }
                }
                self.mark_scroll_to_latest();
            }
            StreamUpdate::Done => {
                let assistant_id = match self.streaming_assistant_id {
                    Some(id) => id,
                    None => return,
                };
                let thread_id = match self.state.thread_id {
                    Some(thread_id) => thread_id,
                    None => return,
                };
                self.state.finish_streaming();
                if let Some(message) = self
                    .state
                    .messages
                    .iter()
                    .find(|message| message.id == assistant_id)
                {
                    let content = message.content.clone();
                    if assistant_id == PENDING_ASSISTANT_ID {
                        if content.trim().is_empty() {
                            self.state.messages.pop();
                        } else if !self.pending_assistant_insert_failed {
                            let inserted_id =
                                self.insert_pending_assistant(thread_id, &content, store);
                            if inserted_id == PENDING_ASSISTANT_ID {
                                self.pending_assistant_insert_failed = true;
                            }
                        }
                    } else {
                        self.persist_assistant_content(assistant_id, &content, store);
                    }
                }
                self.streaming_assistant_id = None;
                self.mark_scroll_to_latest();
            }
        }
    }
}

impl ComposerActions for ChatView {
    fn set_speed_mode(&mut self, mode: crate::api::SpeedMode, cx: &mut Context<Self>) {
        ChatView::set_speed_mode(self, mode, cx);
    }

    fn set_reasoning_effort(&mut self, effort: &str, cx: &mut Context<Self>) {
        ChatView::set_reasoning_effort(self, effort, cx);
    }

    fn on_send_clicked(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        ChatView::on_send_clicked(self, event, window, cx);
    }

    fn on_stop_clicked(&mut self, _event: &ClickEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        self.cancel_active_stream();
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
        ApiError::UnknownModel(model) => model_unavailable_message(&model),
        ApiError::RequestFailed(message) => format!("Request failed: {message}"),
        ApiError::ParseError(message) => format!("Could not read provider response: {message}"),
    }
}

fn catalog_fetched_at_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

impl Focusable for ChatView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_model_select_sync {
            self.pending_model_select_sync = false;
            sync_model_select(
                &self.model_select,
                &self.state.catalog.models,
                &self.state.thread_settings.model_id,
                window,
                cx,
            );
        }

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
        let is_streaming = self.state.is_streaming;
        let streaming_assistant_id = self.streaming_assistant_id;
        let show_credentials_banner = self.credential_ui.should_show_banner();
        let error = self.state.error.clone();
        let selected_model = self
            .state
            .catalog
            .model_for_id(&self.state.thread_settings.model_id);
        let supports_thinking =
            selected_model.is_some_and(|model| model.supports_thinking_controls());
        let catalog_refreshing = self.state.catalog.is_refreshing;

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
            let stream_status = assistant_stream_status(
                message,
                is_streaming,
                streaming_assistant_id,
                supports_thinking,
            );
            let is_dark = self.theme.mode == ThemeMode::Dark;
            thread = thread.child(message_row(
                message,
                stream_status,
                foreground,
                muted,
                user_bubble_bg,
                card_bg,
                label,
                is_dark,
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
            .min_w(px(0.))
            .overflow_hidden()
            .px(inset)
            .pt(inset)
            .pb(inset)
            .child(thread);

        content = content.child(
            div()
                .id("chat-messages-scroll")
                .flex_1()
                .min_h_0()
                .min_w(px(0.))
                .w_full()
                .overflow_x_hidden()
                .track_scroll(&self.message_scroll)
                .overflow_y_scroll()
                .child(message_list)
                .vertical_scrollbar(&self.message_scroll),
        );

        let input = Input::new(&self.input)
            .h(px(64.))
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
                        .overflow_hidden()
                        .child(
                            div()
                                .px(px(12.))
                                .pt(px(10.))
                                .pb(px(4.))
                                .when(!can_send, |this| this.opacity(0.6))
                                .child(input),
                        )
                        .child(render_composer_toolbar(
                            ComposerToolbarProps {
                                model_select: &self.model_select,
                                model: selected_model,
                                messages: &self.state.messages,
                                generation: &self.state.thread_settings.generation,
                                catalog_refreshing,
                                is_streaming,
                                muted,
                                border,
                                can_send,
                            },
                            cx,
                        )),
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
    stream_status: super::stream_indicator::AssistantStreamStatus,
    foreground: gpui::Hsla,
    muted: gpui::Hsla,
    user_bubble_bg: gpui::Hsla,
    assistant_pill_bg: gpui::Hsla,
    label: LegacyTypeRole,
    is_dark: bool,
) -> impl IntoElement {
    let text_size = px(label.size_px as f32);

    match message.role {
        MessageRole::User => div().w_full().flex().justify_end().child(
            div()
                .max_w(relative(0.82))
                .px(px(14.))
                .py(px(10.))
                .rounded_lg()
                .bg(user_bubble_bg)
                .child(bounded_message_text(
                    message.content.clone(),
                    text_size,
                    foreground,
                )),
        ),
        MessageRole::Assistant => div()
            .w_full()
            .min_w(px(0.))
            .overflow_hidden()
            .py(px(4.))
            .pr(px(24.))
            .child(div().w_full().min_w(px(0.)).overflow_hidden().child(
                render_assistant_stream_body(
                    message,
                    stream_status,
                    foreground,
                    muted,
                    assistant_pill_bg,
                    label.size_px as f32,
                    is_dark,
                ),
            )),
        MessageRole::System => div().w_full().flex().justify_center().py(px(8.)).child(
            div()
                .text_size(px(11.))
                .text_color(muted)
                .child(message.content.clone()),
        ),
    }
}
