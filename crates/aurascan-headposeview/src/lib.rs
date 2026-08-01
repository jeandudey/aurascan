use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}

/// Owns only rendering resources. The caller supplies the device, queue, and
/// surface format, and drives resize/render with surface textures it owns.
pub struct Renderer {
    bust_pipeline: wgpu::RenderPipeline,
    bust_vertex_buf: wgpu::Buffer,
    bust_index_buf: wgpu::Buffer,
    bust_index_count: u32,
    bust_uniform_buf: wgpu::Buffer,
    bust_bind_group: wgpu::BindGroup,

    grid_pipeline: wgpu::RenderPipeline,
    grid_vertex_buf: wgpu::Buffer,
    grid_vertex_count: u32,
    grid_uniform_buf: wgpu::Buffer,
    grid_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let bust_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bust-uniform"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bust_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bust_uniform_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let bust_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let grid_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid-uniform"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid-bind-group"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: grid_uniform_buf.as_entire_binding(),
            }],
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main_grid"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let (bust_vertices, bust_indices) = geometry::head(13, 9);

        let bust_vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bust-vtx"),
            contents: bytemuck::cast_slice(&bust_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let bust_index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bust-idx"),
            contents: bytemuck::cast_slice(&bust_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let grid_vertices = geometry::grid(8.0, 16, [0.5, 0.5, 0.5], [0.3, 0.3, 0.3]);
        let grid_vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid-vtx"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            bust_pipeline,
            bust_vertex_buf,
            bust_index_buf,
            bust_index_count: bust_indices.len() as u32,
            bust_uniform_buf,
            bust_bind_group,

            grid_pipeline,
            grid_vertex_buf,
            grid_vertex_count: grid_vertices.len() as u32,
            grid_uniform_buf,
            grid_bind_group,
        }
    }

    /// Renders into the given view. The caller owns the surface texture and is
    /// responsible for acquiring it and presenting afterwards.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        yaw: f32,
        pitch: f32,
        roll: f32,
        aspect: f32,
    ) {
        queue.write_buffer(
            &self.bust_uniform_buf,
            0,
            bytemuck::cast_slice(&[Uniforms {
                mvp: build_mvp(yaw, pitch, roll, aspect),
            }]),
        );
        queue.write_buffer(
            &self.grid_uniform_buf,
            0,
            bytemuck::cast_slice(&[Uniforms {
                mvp: build_vp(aspect).to_cols_array_2d(),
            }]),
        );

        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.bust_pipeline);
            rp.set_bind_group(0, &self.bust_bind_group, &[]);
            rp.set_vertex_buffer(0, self.bust_vertex_buf.slice(..));
            rp.set_index_buffer(self.bust_index_buf.slice(..), wgpu::IndexFormat::Uint16);
            rp.draw_indexed(0..self.bust_index_count, 0, 0..1);

            rp.set_pipeline(&self.grid_pipeline);
            rp.set_bind_group(0, &self.grid_bind_group, &[]);
            rp.set_vertex_buffer(0, self.grid_vertex_buf.slice(..));
            rp.draw(0..self.grid_vertex_count, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}

fn build_vp(aspect: f32) -> Mat4 {
    let proj = Mat4::perspective_rh(60f32.to_radians(), aspect, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 2.0, 6.0), Vec3::ZERO, Vec3::Y);
    let flip_y = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)); // TODO: Feels wrong...
    flip_y * proj * view
}

fn build_mvp(yaw: f32, pitch: f32, roll: f32, aspect: f32) -> [[f32; 4]; 4] {
    let rotation = Quat::from_euler(
        glam::EulerRot::XYZ,
        pitch.to_radians(),
        yaw.to_radians(),
        roll.to_radians(),
    );
    let model = Mat4::from_quat(rotation);
    (build_vp(aspect) * model).to_cols_array_2d()
}

const SHADER: &str = r#"
struct Uniforms { mvp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) color: vec3<f32>) -> VsOut {
    var out: VsOut;
    out.pos = u.mvp * vec4<f32>(pos, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

@fragment
fn fs_main_grid(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 0.55);
}
"#;

mod geometry {
    use super::*;

    pub fn grid(size: f32, divisions: u16, color_mid: [f32; 3], color: [f32; 3]) -> Vec<Vertex> {
        let half = size / 2.0;
        let step = size / divisions as f32;
        let center = divisions / 2;

        let mut vertices = Vec::with_capacity((divisions as usize + 1) * 4);
        for i in 0..=divisions {
            let p = -half + step * i as f32;
            let color = if i == center { color_mid } else { color };

            vertices.push(Vertex {
                pos: [-half, 0.0, p],
                color,
            });
            vertices.push(Vertex {
                pos: [half, 0.0, p],
                color,
            });
            vertices.push(Vertex {
                pos: [p, 0.0, -half],
                color,
            });
            vertices.push(Vertex {
                pos: [p, 0.0, half],
                color,
            });
        }
        vertices
    }

    pub fn head(seg_w: u16, seg_h: u16) -> (Vec<Vertex>, Vec<u16>) {
        let (mut positions, indices) = sphere(seg_w, seg_h);
        deform_head(&mut positions);
        let vertices = positions
            .iter()
            .map(|pos| Vertex {
                pos: pos.to_array(),
                color: [1.0, 1.0, 1.0],
            })
            .collect::<Vec<_>>();
        (vertices, indices)
    }

    pub fn sphere(seg_w: u16, seg_h: u16) -> (Vec<Vec3>, Vec<u16>) {
        let mut positions = Vec::new();
        let mut indices = Vec::new();

        for y in 0..=seg_h {
            let theta = (y as f32 / seg_h as f32) * std::f32::consts::PI;
            for x in 0..=seg_w {
                let phi = (x as f32 / seg_w as f32) * std::f32::consts::TAU;
                positions.push(Vec3::new(
                    -phi.cos() * theta.sin(),
                    theta.cos(),
                    phi.sin() * theta.sin(),
                ));
            }
        }

        let row = seg_w + 1;
        for y in 0..seg_h {
            for x in 0..seg_w {
                let a = y * row + x + 1;
                let b = y * row + x;
                let c = (y + 1) * row + x;
                let d = (y + 1) * row + x + 1;
                if y != 0 {
                    indices.extend_from_slice(&[a, d, b]);
                }
                if y != seg_h - 1 {
                    indices.extend_from_slice(&[b, d, c]);
                }
            }
        }

        (positions, indices)
    }

    fn deform_head(positions: &mut [Vec3]) {
        for pos in positions {
            pos.y *= 1.16;
            if pos.y < 0.0 {
                let t = (1.0 + pos.y * 0.30).max(0.55);
                pos.x *= t;
                pos.z *= t;
            }
            if pos.y < -0.45 {
                pos.z += (-pos.y - 0.45) * 0.30;
            }
            pos.z *= 1.04;
        }
    }
}
