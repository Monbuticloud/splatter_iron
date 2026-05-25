use std::io::Write;
use std::path::Path;
use zstd;

use crate::canvas::Canvas;

const COMPRESSION_LEVEL: i32 = 10;

pub fn get_save_data(canvas: &Canvas) -> anyhow::Result<Vec<u8>> {
    let json = serde_json::to_vec(canvas)?;
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), COMPRESSION_LEVEL)?;
    encoder.multithread(n_threads)?;
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

pub fn save_data_to_file(data: &[u8], path: &Path) -> Result<(), std::io::Error> {
    std::fs::write(path, data)?;
    Ok(())
}

pub fn load_data_from_file(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

pub fn load_app_from_data(data: &[u8]) -> anyhow::Result<Canvas> {
    let decompressed = zstd::decode_all(data)?;
    let canvas = serde_json::from_slice(&decompressed)?;
    Ok(canvas)
}

pub fn save_canvas(app: &crate::app::MyApp) -> anyhow::Result<()> {
    let data = get_save_data(&app.canvas)?;
    save_data_to_file(&data, Path::new(&app.savefile_path))?;
    Ok(())
}