//! Brush library — alias for [`Library<BrushEntry>`].

use eframe::egui::Color32;
use eframe::egui::TextureHandle;
use eframe::egui::{self};

use crate::asset_library::AssetEntry;
use crate::asset_library::Library;

/// A single brush entry in the library.

pub struct BrushEntry {
    pub name: String,
    pub filename: String,
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    /// Spacing percentage (0–100) from the original brush file.
    pub spacing: u8,
    pub texture_handle: Option<TextureHandle>,
}

impl std::fmt::Debug for BrushEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        f.debug_struct("BrushEntry")
            .field("name", &self.name)
            .field("filename", &self.filename)
            .field("pixels.len", &self.pixels.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("spacing", &self.spacing)
            .finish()
    }
}

impl BrushEntry {
    /// Return the egui texture ID for this brush's preview thumbnail.
    ///
    /// Returns `None` if the texture has not been created yet
    /// (e.g. before the first call to [`Library::create_textures`]).

    pub fn texture_id(&self) -> Option<egui::TextureId> {

        self.texture_handle.as_ref().map(TextureHandle::id)
    }
}

impl AssetEntry for BrushEntry {
    fn name(&self) -> &str {

        &self.name
    }

    fn filename(&self) -> &str {

        &self.filename
    }

    fn filename_mut(&mut self) -> &mut String {

        &mut self.filename
    }

    fn pixels(&self) -> &[Color32] {

        &self.pixels
    }

    fn width(&self) -> u32 {

        self.width
    }

    fn height(&self) -> u32 {

        self.height
    }

    fn texture_handle(&self) -> &Option<TextureHandle> {

        &self.texture_handle
    }

    fn texture_handle_mut(&mut self) -> &mut Option<TextureHandle> {

        &mut self.texture_handle
    }

    fn dir_name() -> &'static str {

        "brushes"
    }

    fn json_field_name() -> &'static str {

        "brushes"
    }

    fn extra_index_fields(&self) -> Vec<(&'static str, serde_json::Value)> {

        vec![(
            "spacing",
            serde_json::Value::Number(serde_json::Number::from(self.spacing)),
        )]
    }

    fn from_parts(
        name: String,
        filename: String,
        pixels: Vec<Color32>,
        w: u32,
        h: u32,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Self {

        let spacing = extra
            .get("spacing")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(25) as u8;

        Self {
            name,
            filename,
            pixels,
            width: w,
            height: h,
            spacing,
            texture_handle: None,
        }
    }
}

/// Persistent collection of brush tips.

pub type BrushLibrary = Library<BrushEntry>;

/// Create and add a brush entry to the library.

pub fn add_brush(
    lib: &mut BrushLibrary,
    name: String,
    pixels: Vec<Color32>,
    width: u32,
    height: u32,
    spacing: u8,
    ctx: &egui::Context,
) {

    let entry = BrushEntry {
        name,
        filename: String::new(),
        pixels,
        width,
        height,
        spacing,
        texture_handle: None,
    };

    lib.add_entry(entry, ctx);
}
