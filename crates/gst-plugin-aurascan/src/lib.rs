use gst::glib;

pub mod bytetracker;
pub mod detectioncropmeta;
pub mod headposeinferencebin;
pub mod scrfd;
pub mod scrfdtensordec;
pub mod sixdrepnet360;
pub mod sixdrepnet360tensordec;
pub mod videocropscale;

#[derive(Copy, Clone, Default, PartialEq, Eq, glib::Enum)]
#[repr(C)]
#[enum_type(name = "GstBurnExtraBackendType")]
pub enum BackendType {
    #[default]
    Flex = 0,
    #[cfg(feature = "vulkan")]
    Vulkan = 1,
    #[cfg(feature = "rocm")]
    Rocm = 2,
}

fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    if !gst::meta::CustomMeta::is_registered("EulerAnglesMeta") {
        gst::meta::CustomMeta::register("EulerAnglesMeta", &[]);
    }

    bytetracker::register(plugin)?;
    detectioncropmeta::register(plugin)?;
    scrfd::register(plugin)?;
    scrfdtensordec::register(plugin)?;
    sixdrepnet360::register(plugin)?;
    sixdrepnet360tensordec::register(plugin)?;
    videocropscale::register(plugin)?;

    headposeinferencebin::register(plugin)?;

    Ok(())
}

gst::plugin_define!(
    aurascan,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
    "LGPL",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    env!("BUILD_REL_DATE")
);
