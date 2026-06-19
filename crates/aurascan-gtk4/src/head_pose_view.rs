use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use aurascan_headposeview::Renderer;

    use crate::WGPUArea;

    use super::*;

    #[derive(Default)]
    pub struct HeadPoseView {
        renderer: OnceCell<Renderer>,
        rotation: RefCell<(f32, f32, f32)>,
        aspect_ratio: RefCell<f32>,
        wgpu_area: WGPUArea,
    }

    impl HeadPoseView {
        pub fn set_rotation(&self, yaw: f32, pitch: f32, roll: f32) {
            *self.rotation.borrow_mut() = (yaw, pitch, roll);
            self.wgpu_area.queue_render();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HeadPoseView {
        const NAME: &'static str = "AscHeadPoseView";
        type Type = super::HeadPoseView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }
    }

    impl ObjectImpl for HeadPoseView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.wgpu_area.set_auto_render(false);

            self.wgpu_area.connect_resize(glib::clone!(
                #[weak]
                obj,
                move |_wgpu_area, width, height| {
                    *obj.imp().aspect_ratio.borrow_mut() = width as f32 / height as f32;
                }
            ));

            self.wgpu_area.connect_render_wgpu(glib::clone!(
                #[weak]
                obj,
                move |wgpu_area| {
                    let renderer = obj.imp().renderer.get_or_init(|| {
                        Renderer::new(
                            &wgpu_area.device().unwrap(),
                            wgpu_area.color_view().unwrap().texture().format(),
                            wgpu_area.depth_view().unwrap().texture().format(),
                        )
                    });

                    let (yaw, pitch, roll) = *obj.imp().rotation.borrow();
                    renderer.render(
                        &wgpu_area.device().unwrap(),
                        &wgpu_area.queue().unwrap(),
                        &wgpu_area.color_view().unwrap(),
                        &wgpu_area.depth_view().unwrap(),
                        yaw,
                        pitch,
                        roll,
                        *obj.imp().aspect_ratio.borrow(),
                    );
                }
            ));

            self.wgpu_area.set_parent(&*obj);
        }

        fn dispose(&self) {
            self.wgpu_area.unparent();
        }
    }

    impl WidgetImpl for HeadPoseView {}
}

glib::wrapper! {
    pub struct HeadPoseView(ObjectSubclass<imp::HeadPoseView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for HeadPoseView {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl HeadPoseView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_rotation(&self, yaw: f32, pitch: f32, roll: f32) {
        self.imp().set_rotation(yaw, pitch, roll);
    }
}
