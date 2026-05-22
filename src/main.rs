mod app;
mod canvas;
mod pixel;
mod undo;

// use mimalloc::MiMalloc;

// #[global_allocator]
// static GLOBAL: MiMalloc = MiMalloc;
// Unstable MiMalloc features can cause build issues, so we'll stick to the default allocator for now.

fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app::MyApp::default())))
    )
}
