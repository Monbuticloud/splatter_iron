//! Generic persistent asset library with on-disk storage via PNG files + JSON index.
//!
//! Provides [`Library<T>`] parameterised over an [`AssetEntry`] type.
//! Brush and stamp libraries are thin wrappers around this generic.

use std::path::Path;

use eframe::egui::Color32;
use eframe::egui::TextureHandle;
use eframe::egui::{self};
use serde_json;

use crate::pixel;

/// Behaviour that an asset entry must implement for storage in a [`Library`].
pub trait AssetEntry: Sized {
    /// Display name.
    fn name(&self) -> &str;
    /// Mutable reference to the display name.
    fn name_mut(&mut self) -> &mut String;
    /// On-disk PNG filename (relative to the library directory).
    fn filename(&self) -> &str;
    /// Mutable reference to the on-disk PNG filename.
    fn filename_mut(&mut self) -> &mut String;
    /// Premultiplied-alpha pixel data (row-major).
    fn pixels(&self) -> &[Color32];
    /// Mutable reference to the premultiplied-alpha pixel buffer.
    fn pixels_mut(&mut self) -> &mut Vec<Color32>;
    /// Image dimensions in pixels.
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    /// Cached egui texture for preview rendering.
    fn texture_handle(&self) -> &Option<TextureHandle>;
    /// Mutable reference to the cached egui preview texture.
    fn texture_handle_mut(&mut self) -> &mut Option<TextureHandle>;

    /// Name of the subdirectory within the data directory (e.g. `"brushes"`).
    fn dir_name() -> &'static str;
    /// JSON field name for the entry array in the index file (e.g. `"brushes"`).
    fn json_field_name() -> &'static str;

    /// Extra key-value pairs to serialise into the index-file entry.
    fn extra_index_fields(&self) -> Vec<(&'static str, serde_json::Value)>;

    /// Reconstruct an entry from index-file data + decoded PNG pixels.
    fn from_parts(
        name: String,
        filename: String,
        pixels: Vec<Color32>,
        w: u32,
        h: u32,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Self;
}

/// Persistent collection of assets with on-disk storage.
pub struct Library<T: AssetEntry> {
    entries: Vec<T>,
    selected_index: Option<usize>,
    dir: std::path::PathBuf,
}

impl<T: AssetEntry + std::fmt::Debug> std::fmt::Debug for Library<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library")
            .field("entries", &self.entries)
            .field("selected_index", &self.selected_index)
            .field("dir", &self.dir)
            .finish()
    }
}

impl<T: AssetEntry> Library<T> {
    /// Load or create a library rooted at `data_dir / T::dir_name()`.
    ///
    /// If the directory does not exist it is created. Entries are loaded
    /// from `index.json`; missing or corrupt PNG files are silently skipped.
    pub fn load_from_disk(data_dir: &Path) -> Self {
        let dir = data_dir.join(T::dir_name());
        std::fs::create_dir_all(&dir).expect("Failed to create asset directory");

        let mut entries: Vec<T> = Vec::new();

        let index_path = dir.join("index.json");
        if index_path.exists() {
            if let Ok(json) = std::fs::read_to_string(&index_path) {
                if let Ok(root) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(arr) = root
                        .get(T::json_field_name())
                        .and_then(|v| v.as_array())
                    {
                        for item in arr {
                            let name = item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let filename = item
                                .get("filename")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let w = item
                                .get("w")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;
                            let h = item
                                .get("h")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;

                            let png_path = dir.join(&filename);
                            if let Ok(img) = image::open(&png_path) {
                                let rgba = img.to_rgba8();
                                let (img_w, img_h) = rgba.dimensions();
                                let mut pixels =
                                    Vec::with_capacity((img_w * img_h) as usize);
                                for pixel in rgba.pixels() {
                                    let straight = Color32::from_rgba_unmultiplied(
                                        pixel[0],
                                        pixel[1],
                                        pixel[2],
                                        pixel[3],
                                    );
                                    pixels.push(straight);
                                }
                                let extra = item
                                    .as_object()
                                    .cloned()
                                    .unwrap_or_default();
                                entries.push(T::from_parts(
                                    name, filename, pixels, w, h, &extra,
                                ));
                            }
                        }
                    }
                }
            }
        }

        let selected_index = if entries.is_empty() { None } else { Some(0) };
        Self {
            entries,
            selected_index,
            dir,
        }
    }

    /// Create egui textures for all entries that don't have one yet.
    pub fn create_textures(&mut self, ctx: &egui::Context) {
        for entry in &mut self.entries {
            if entry.texture_handle().is_none() {
                let raw: Vec<u8> = entry
                    .pixels()
                    .iter()
                    .flat_map(|c| pixel::unpremultiply(*c).to_array())
                    .collect();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [entry.width() as usize, entry.height() as usize],
                    &raw,
                );
                let tex =
                    ctx.load_texture(entry.name(), image, egui::TextureOptions::LINEAR);
                *entry.texture_handle_mut() = Some(tex);
            }
        }
    }

    /// Add an entry to the library, persist its PNG to disk, and update the index.
    pub fn add_entry(&mut self, mut entry: T, ctx: &egui::Context) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("{ts}.png");

        let raw: Vec<u8> = entry
            .pixels()
            .iter()
            .flat_map(|c| pixel::unpremultiply(*c).to_array())
            .collect();
        let _ = image::save_buffer(
            &self.dir.join(&filename),
            &raw,
            entry.width(),
            entry.height(),
            image::ColorType::Rgba8,
        );

        let image = egui::ColorImage::from_rgba_unmultiplied(
            [entry.width() as usize, entry.height() as usize],
            &raw,
        );
        let name = entry.name().to_string();
        let tex = ctx.load_texture(&name, image, egui::TextureOptions::LINEAR);
        *entry.filename_mut() = filename;
        *entry.texture_handle_mut() = Some(tex);

        self.entries.push(entry);
        self.selected_index = Some(self.entries.len() - 1);
        self.save_index();
    }

    /// Remove the entry at `index`, delete its PNG file, and persist the index.
    pub fn remove(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }

        let filename = self.entries[index].filename().to_string();
        self.entries.remove(index);

        let _ = std::fs::remove_file(&self.dir.join(&filename));

        if let Some(sel) = self.selected_index {
            if sel == index {
                self.selected_index = if self.entries.is_empty() {
                    None
                } else {
                    Some(0)
                };
            } else if sel > index {
                self.selected_index = Some(sel - 1);
            }
        }

        self.save_index();
    }

    /// Select the entry at `index`.
    pub fn select(&mut self, index: usize) {
        if index < self.entries.len() {
            self.selected_index = Some(index);
        }
    }

    /// Index of the currently selected entry, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Reference to the currently selected entry, if any.
    pub fn selected(&self) -> Option<&T> {
        self.selected_index.map(|i| &self.entries[i])
    }

    /// Mutable reference to the currently selected entry.
    pub fn selected_mut(&mut self) -> Option<&mut T> {
        self.selected_index.map(|i| &mut self.entries[i])
    }

    /// Slice of all entries.
    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    /// Number of entries in the library.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entry at `index`, or `None`.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.entries.get(index)
    }

    /// Persist the index file to disk.
    fn save_index(&self) {
        let items: Vec<serde_json::Value> = self
            .entries
            .iter()
            .map(|entry| {
                let mut map = serde_json::Map::new();
                map.insert(
                    "name".to_string(),
                    serde_json::Value::String(entry.name().to_string()),
                );
                map.insert(
                    "filename".to_string(),
                    serde_json::Value::String(entry.filename().to_string()),
                );
                map.insert(
                    "w".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(entry.width())),
                );
                map.insert(
                    "h".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(entry.height())),
                );
                for (key, val) in entry.extra_index_fields() {
                    map.insert(key.to_string(), val);
                }
                serde_json::Value::Object(map)
            })
            .collect();

        let root = serde_json::json!({ T::json_field_name(): items });
        let path = self.dir.join("index.json");
        if let Ok(json) = serde_json::to_string_pretty(&root) {
            let _ = std::fs::write(&path, json);
        }
    }
}
