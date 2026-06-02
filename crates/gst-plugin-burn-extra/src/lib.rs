use gst::glib;

pub mod scrfd;
pub mod scrfdtensordec;

pub use gstburn::BackendType;

fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    scrfd::register(plugin)?;
    scrfdtensordec::register(plugin)?;
    Ok(())
}

gst::plugin_define!(
    burnextra,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
    "LGPL",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    env!("BUILD_REL_DATE")
);
