//! Stamp library — thin wrapper around [`Library<StampEntry>`].
//!
//! Re-exports [`StampSampling`] and [`StampTintMode`] from
//! `tool_configuration` for callers that previously imported them here.

use std::path::Path;

use eframe::egui::Color32;
use eframe::egui::TextureHandle;
use eframe::egui::{self};

pub use crate::tool_configuration::StampSampling;
pub use crate::tool_configuration::StampTintMode;

use crate::asset_library::AssetEntry;
use crate::asset_library::Library;

/// A single stamp entry in the library.
pub struct StampEntry {
    pub name: String,
    pub filename: String,
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    pub texture_handle: Option<TextureHandle>,
}

impl StampEntry {
    pub fn texture_id(&self) -> Option<egui::TextureId> {
        self.texture_handle.as_ref().map(|h| h.id())
    }
}

impl AssetEntry for StampEntry {
    fn name(&self) -> &str { &self.name }
    fn name_mut(&mut self) -> &mut String { &mut self.name }
    fn filename(&self) -> &str { &self.filename }
    fn filename_mut(&mut self) -> &mut String { &mut self.filename }
    fn pixels(&self) -> &[Color32] { &self.pixels }
    fn pixels_mut(&mut self) -> &mut Vec<Color32> { &mut self.pixels }
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn texture_handle(&self) -> &Option<TextureHandle> { &self.texture_handle }
    fn texture_handle_mut(&mut self) -> &mut Option<TextureHandle> { &mut self.texture_handle }

    fn dir_name() -> &'static str { "stamps" }
    fn json_field_name() -> &'static str { "stamps" }

    fn extra_index_fields(&self) -> Vec<(&'static str, serde_json::Value)> {
        Vec::new()
    }

    fn from_parts(
        name: String,
        filename: String,
        pixels: Vec<Color32>,
        w: u32,
        h: u32,
        _extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            name,
            filename,
            pixels,
            width: w,
            height: h,
            texture_handle: None,
        }
    }
}

/// Persistent collection of stamp images.
pub struct StampLibrary(pub Library<StampEntry>);

impl StampLibrary {
    pub fn load_from_disk(data_dir: &Path) -> Self {
        Self(Library::load_from_disk(data_dir))
    }

    pub fn create_textures(&mut self, ctx: &egui::Context) {
        self.0.create_textures(ctx);
    }

    pub fn add(
        &mut self,
        name: String,
        pixels: Vec<Color32>,
        width: u32,
        height: u32,
        ctx: &egui::Context,
    ) {
        let entry = StampEntry {
            name,
            filename: String::new(),
            pixels,
            width,
            height,
            texture_handle: None,
        };
        self.0.add_entry(entry, ctx);
    }

    pub fn remove(&mut self, index: usize) { self.0.remove(index); }
    pub fn select(&mut self, index: usize) { self.0.select(index); }
    pub fn selected_index(&self) -> Option<usize> { self.0.selected_index() }
    pub fn selected(&self) -> Option<&StampEntry> { self.0.selected() }
    pub fn selected_mut(&mut self) -> Option<&mut StampEntry> { self.0.selected_mut() }
    pub fn entries(&self) -> &[StampEntry] { self.0.entries() }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn get(&self, index: usize) -> Option<&StampEntry> { self.0.get(index) }
}
