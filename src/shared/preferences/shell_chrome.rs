//! Persisted shell chrome sizes and open flags.

pub const TITLEBAR_HEIGHT: f32 = 38.0;
pub const SIDEBAR_MIN: f32 = 208.0;
pub const SIDEBAR_MAX: f32 = 400.0;
pub const SIDEBAR_DEFAULT: f32 = 256.0;
pub const RIGHT_MIN: f32 = 240.0;
pub const RIGHT_MAX: f32 = 480.0;
pub const RIGHT_DEFAULT: f32 = 320.0;
pub const BOTTOM_MIN: f32 = 120.0;
pub const BOTTOM_DEFAULT: f32 = 220.0;
pub const BOTTOM_MAX_VH: f32 = 0.55;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ShellTabRecord {
    pub id: String,
    pub title: String,
}

impl Default for ShellTabRecord {
    fn default() -> Self {
        Self {
            id: "tab-1".into(),
            title: "Welcome".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ShellChrome {
    pub left_width: f32,
    pub right_width: f32,
    pub bottom_height: f32,
    pub left_open: bool,
    pub right_open: bool,
    pub bottom_open: bool,
    pub tabs: Vec<ShellTabRecord>,
    pub active_tab_id: String,
}

impl Default for ShellChrome {
    fn default() -> Self {
        let tab = ShellTabRecord::default();
        Self {
            left_width: SIDEBAR_DEFAULT,
            right_width: RIGHT_DEFAULT,
            bottom_height: BOTTOM_DEFAULT,
            left_open: true,
            right_open: false,
            bottom_open: false,
            active_tab_id: tab.id.clone(),
            tabs: vec![tab],
        }
    }
}

pub fn clamp_sidebar_width(w: f32) -> f32 {
    w.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
}

pub fn clamp_right_width(w: f32, viewport_w: f32) -> f32 {
    let max = RIGHT_MAX.min(viewport_w * 0.52);
    w.clamp(RIGHT_MIN, max.max(RIGHT_MIN))
}

pub fn clamp_bottom_height(h: f32, viewport_h: f32) -> f32 {
    let max = (viewport_h * BOTTOM_MAX_VH).max(BOTTOM_MIN);
    h.clamp(BOTTOM_MIN, max)
}

impl ShellChrome {
    /// Clamp all sizes against a viewport; repair empty tabs.
    pub fn sanitized(mut self, viewport_w: f32, viewport_h: f32) -> Self {
        self.left_width = clamp_sidebar_width(self.left_width);
        self.right_width = clamp_right_width(self.right_width, viewport_w);
        self.bottom_height = clamp_bottom_height(self.bottom_height, viewport_h);
        if self.tabs.is_empty() {
            let tab = ShellTabRecord::default();
            self.active_tab_id = tab.id.clone();
            self.tabs.push(tab);
        }
        if !self.tabs.iter().any(|t| t.id == self.active_tab_id) {
            self.active_tab_id = self.tabs[0].id.clone();
        }
        self
    }
}
