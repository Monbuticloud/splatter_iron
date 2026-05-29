//! Brush library: persistent collection of custom brush tips with naming,
//! thumbnails, and on-disk storage via PNG files + JSON index.

use std::path::Path;

use eframe::egui::{self, Color32, TextureHandle};
use serde::{Deserialize, Serialize};

const BRUSHES_DIR_NAME: &str = "brushes";
const INDEX_FILE_NAME: &str = "index.json";

/// A single brush entry in the library.
pub struct BrushEntry {
    /// User-given display name.
    pub name: String,
    /// On-disk PNG filename (relative to `brushes/` directory).
    pub filename: String,
    /// Premultiplied-alpha pixel data (row-major).
    pub pixels: Vec<Color32>,
    /// Width of the brush tip in pixels.
    pub width: u32,
    /// Height of the brush tip in pixels.
    pub height: u32,
    /// Spacing percentage (0–100) from the original brush file.
    pub spacing: u8,
    /// Cached egui texture for preview rendering.
    pub texture_handle: Option<TextureHandle>,
}

impl BrushEntry {
    /// Return the cached texture ID, if a texture has been created.
    pub fn texture_id(&self) -> Option<egui::TextureId> {
        self.texture_handle.as_ref().map(|h| h.id())
    }
}

// ---- Serialization schema for the index file ----

#[derive(Serialize, Deserialize)]
struct IndexFile {
    brushes: Vec<IndexEntry>,
}

#[derive(Serialize, Deserialize)]
struct IndexEntry {
    name: String,
    filename: String,
    w: u32,
    h: u32,
    spacing: u8,
}

/// Persistent collection of brush tips with on-disk storage.
pub struct BrushLibrary {
    /// Stored brush entries.
    brushes: Vec<BrushEntry>,
    /// Index of the currently selected brush, if any.
    selected_index: Option<usize>,
    /// Absolute path to the brushes directory on disk.
    brushes_dir: std::path::PathBuf,
}

impl BrushLibrary {
    /// Load or create a brush library rooted at `data_dir/brushes/`.
    ///
    /// If the directory does not exist it is created.  Entries are loaded
    /// from `index.json`; missing or corrupt PNG files are silently skipped.
    ///
    /// # Parameters
    ///
    /// * `data_dir` — Application data directory (parent of the `brushes/` subdir).
    ///
    /// # Panics
    ///
    /// Panics if the brushes directory cannot be created.
    pub fn load_from_disk(data_dir: &Path) -> Self {
        let brushes_dir = data_dir.join(BRUSHES_DIR_NAME);
        std::fs::create_dir_all(&brushes_dir).expect("Failed to create brushes directory");

        let mut brushes: Vec<BrushEntry> = Vec::new();

        let index_path = brushes_dir.join(INDEX_FILE_NAME);
        if index_path.exists() {
            if let Ok(json) = std::fs::read_to_string(&index_path) {
                if let Ok(index) = serde_json::from_str::<IndexFile>(&json) {
                    for entry in index.brushes {
                        let png_path = brushes_dir.join(&entry.filename);
                        if let Ok(img) = image::open(&png_path) {
                            let rgba = img.to_rgba8();
                            let (w, h) = rgba.dimensions();
                            let mut pixels = Vec::with_capacity((w * h) as usize);
                            for pixel in rgba.pixels() {
                                let straight = Color32::from_rgba_unmultiplied(
                                    pixel[0], pixel[1], pixel[2], pixel[3],
                                );
                                pixels.push(straight);
                            }
                            brushes.push(BrushEntry {
                                name: entry.name,
                                filename: entry.filename,
                                pixels,
                                width: w,
                                height: h,
                                spacing: entry.spacing,
                                texture_handle: None,
                            });
                        }
                    }
                }
            }
        }

        let selected_index = if brushes.is_empty() { None } else { Some(0) };
        Self { brushes, selected_index, brushes_dir }
    }

    /// Create egui textures for all brush entries that don't have one yet.
    ///
    /// Should be called after loading or when the egui context is available.
    ///
    /// # Parameters
    ///
    /// * `ctx` — The egui context for texture creation.
    pub fn create_textures(&mut self, ctx: &egui::Context) {
        for entry in &mut self.brushes {
            if entry.texture_handle.is_none() {
                let raw: Vec<u8> = entry
                    .pixels
                    .iter()
                    .flat_map(|c| crate::pixel::unpremultiply(*c).to_array())
                    .collect();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [entry.width as usize, entry.height as usize],
                    &raw,
                );
                entry.texture_handle = Some(
                    ctx.load_texture(&entry.name, image, egui::TextureOptions::LINEAR),
                );
            }
        }
    }

    /// Add a new brush to the library and persist it to disk.
    ///
    /// Creates a PNG file in the brushes directory and updates `index.json`.
    /// A cached texture handle is created for preview rendering.
    ///
    /// # Parameters
    ///
    /// * `name` — Display name for the brush.
    /// * `pixels` — Premultiplied-alpha pixel data.
    /// * `width` — Image width in pixels.
    /// * `height` — Image height in pixels.
    /// * `spacing` — Spacing percentage (0–100).
    /// * `ctx` — Egui context for texture creation.
    pub fn add(
        &mut self,
        name: String,
        pixels: Vec<Color32>,
        width: u32,
        height: u32,
        spacing: u8,
        ctx: &egui::Context,
    ) {
        // Generate a unique filename from a nanosecond timestamp
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("{ts}.png");

        // Un-premultiply and save as PNG
        let raw: Vec<u8> = pixels
            .iter()
            .flat_map(|c| crate::pixel::unpremultiply(*c).to_array())
            .collect();
        let png_path = self.brushes_dir.join(&filename);
        let _ = image::save_buffer(&png_path, &raw, width, height, image::ColorType::Rgba8);

        // Create egui texture for preview
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            &raw,
        );
        let tex = ctx.load_texture(&name, image, egui::TextureOptions::LINEAR);

        let entry = BrushEntry {
            name,
            filename,
            pixels,
            width,
            height,
            spacing,
            texture_handle: Some(tex),
        };

        self.brushes.push(entry);
        self.selected_index = Some(self.brushes.len() - 1);

        self.save_index();
    }

    /// Remove the brush at `index` from the library, delete its PNG file,
    /// and persist the updated index.
    ///
    /// If `index` matches the current selection, selection is cleared.
    /// The cached texture is freed automatically when the entry is dropped.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the entry to remove.
    pub fn remove(&mut self, index: usize) {
        if index >= self.brushes.len() {
            return;
        }

        let filename = self.brushes[index].filename.clone();
        self.brushes.remove(index);

        // Remove associated PNG file from disk
        let png_path = self.brushes_dir.join(&filename);
        let _ = std::fs::remove_file(&png_path);

        // Adjust selection
        if let Some(sel) = self.selected_index {
            if sel == index {
                self.selected_index = if self.brushes.is_empty() { None } else { Some(0) };
            } else if sel > index {
                self.selected_index = Some(sel - 1);
            }
        }

        self.save_index();
    }

    /// Select the brush at `index`.
    pub fn select(&mut self, index: usize) {
        if index < self.brushes.len() {
            self.selected_index = Some(index);
        }
    }

    /// Return the index of the currently selected brush, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Return a reference to the currently selected brush entry, if any.
    pub fn selected(&self) -> Option<&BrushEntry> {
        self.selected_index.map(|i| &self.brushes[i])
    }

    /// Return a slice of all brush entries.
    pub fn entries(&self) -> &[BrushEntry] {
        &self.brushes
    }

    /// Return the number of brushes in the library.
    pub fn len(&self) -> usize {
        self.brushes.len()
    }

    /// Return `true` if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.brushes.is_empty()
    }

    /// Return a reference to the brush at `index`, or `None`.
    pub fn get(&self, index: usize) -> Option<&BrushEntry> {
        self.brushes.get(index)
    }

    /// Persist the current library index to disk.
    fn save_index(&self) {
        let index = IndexFile {
            brushes: self
                .brushes
                .iter()
                .map(|entry| IndexEntry {
                    name: entry.name.clone(),
                    filename: entry.filename.clone(),
                    w: entry.width,
                    h: entry.height,
                    spacing: entry.spacing,
                })
                .collect(),
        };
        let path = self.brushes_dir.join(INDEX_FILE_NAME);
        if let Ok(json) = serde_json::to_string_pretty(&index) {
            let _ = std::fs::write(&path, json);
        }
    }
}
