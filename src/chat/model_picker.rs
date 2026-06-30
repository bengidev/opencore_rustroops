//! Header model picker backed by cached catalog entries.

use std::sync::Arc;

use gpui::{App, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::Sizable;
use gpui_component::select::{SearchableVec, Select, SelectState};

use crate::api::ModelInfo;

use super::chat_store::{ChatStore, ThreadSettings};

/// Select list entry wrapping normalized [`ModelInfo`].
#[derive(Debug, Clone)]
pub struct ModelSelectEntry {
    pub info: ModelInfo,
}

impl gpui_component::select::SelectItem for ModelSelectEntry {
    type Value = String;

    fn title(&self) -> SharedString {
        SharedString::from(self.info.name.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.info.id
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.info.name.to_lowercase().contains(&query)
            || self.info.id.to_lowercase().contains(&query)
    }
}

pub fn entries_from_models(models: &[ModelInfo]) -> SearchableVec<ModelSelectEntry> {
    SearchableVec::new(
        models
            .iter()
            .cloned()
            .map(|info| ModelSelectEntry { info })
            .collect::<Vec<_>>(),
    )
}

pub fn selected_index_for_model(
    models: &[ModelInfo],
    model_id: &str,
) -> Option<gpui_component::IndexPath> {
    models
        .iter()
        .position(|model| model.id == model_id)
        .map(|index| gpui_component::IndexPath::default().row(index))
}

pub fn sync_model_select(
    select: &Entity<SelectState<SearchableVec<ModelSelectEntry>>>,
    models: &[ModelInfo],
    model_id: &str,
    window: &mut Window,
    cx: &mut App,
) {
    select.update(cx, |state, cx| {
        state.set_items(entries_from_models(models), window, cx);
        state.set_selected_value(&model_id.to_string(), window, cx);
    });
}

pub fn render_model_select(
    select: &Entity<SelectState<SearchableVec<ModelSelectEntry>>>,
) -> Select<SearchableVec<ModelSelectEntry>> {
    Select::new(select).placeholder("Model").small()
}

/// Inline model picker for the composer footer row.
pub fn render_composer_model_select(
    select: &Entity<SelectState<SearchableVec<ModelSelectEntry>>>,
) -> impl IntoElement {
    div().flex_shrink_0().max_w(px(180.)).min_w(px(96.)).child(
        render_model_select(select)
            .appearance(false)
            .small()
            .menu_width(px(320.)),
    )
}

pub fn persist_model_selection(
    store: &Arc<dyn ChatStore>,
    thread_id: i64,
    settings: &mut ThreadSettings,
    model_id: String,
) -> Result<(), String> {
    settings.model_id = model_id;
    store
        .save_thread_settings(thread_id, settings)
        .map_err(|error| error.to_string())
}
