use gtk::prelude::*;

mod preferences_window;
mod window;

pub use preferences_window::PreferencesWindow;
pub use window::Window;

pub fn init() {
    PreferencesWindow::static_type();
    Window::static_type();
}
