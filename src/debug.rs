use crate::app::MyApp;
use crate::main;
pub fn debug_snapshot(app: &MyApp) {
    #[cfg(debug_assertions)]
    {
        dbg!(app);
    }
}
