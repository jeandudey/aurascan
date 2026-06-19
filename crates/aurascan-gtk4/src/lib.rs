use std::sync::Once;

mod head_pose_view;
mod wgpu_area;

use gtk::glib::types::StaticType;
pub use head_pose_view::HeadPoseView;
pub use wgpu_area::WGPUArea;

static IS_INIT: Once = Once::new();

pub fn init() {
    IS_INIT.call_once(|| {
        gtk::init().unwrap();

        HeadPoseView::static_type();
        WGPUArea::static_type();
    });
}
