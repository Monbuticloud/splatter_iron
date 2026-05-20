mod app;
mod canvas;
mod pixel;

fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app::MyApp::default()))),
    )
}