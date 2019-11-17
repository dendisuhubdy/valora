use crate::{
    raster::{raster_path, Method},
    Result,
    SampleDepth,
    V4,
};
use glium::{
    backend::glutin::headless::Headless,
    implement_vertex,
    index::PrimitiveType,
    texture::{
        texture2d::Texture2d,
        texture2d_multisample::Texture2dMultisample,
        MipmapsOption,
        RawImage2d,
        UncompressedFloatFormat,
    },
    uniforms::{MagnifySamplerFilter, UniformValue, Uniforms},
    Blend,
    BlendingFunction,
    DrawParameters,
    IndexBuffer,
    LinearBlendingFactor,
    Program,
    Surface,
    VertexBuffer,
};
use glutin::dpi::PhysicalSize;
use itertools::Itertools;
use lyon_path::Builder;
use rand::random;
use std::rc::Rc;

#[derive(Debug, Copy, Clone)]
pub struct GpuVertex {
    pub vpos: [f32; 2],
    pub vcol: [f32; 4],
}

implement_vertex!(GpuVertex, vpos, vcol);

pub const VERTEX_SHADER: &str = include_str!("shaders/default.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/default.frag");

#[derive(Clone)]
pub struct Shader {
    id: u64,
    program: Rc<Program>,
    uniforms: UniformBuffer,
}

#[derive(Default, Clone)]
pub struct UniformBuffer {
    uniforms: Vec<(String, UniformValue<'static>)>,
}

impl UniformBuffer {
    pub fn push(&mut self, name: String, value: UniformValue<'static>) {
        self.uniforms.push((name, value));
    }
}

impl Uniforms for UniformBuffer {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut f: F) {
        for (name, value) in &self.uniforms {
            f(name.as_str(), *value);
        }
    }
}

/// A rasterable element in a composition.
pub struct Element {
    pub path: Builder,
    pub color: V4,
    pub raster_method: Method,
    pub shader: Shader,
    pub sample_depth: SampleDepth,
}

pub struct Gpu {
    ctx: Rc<Headless>,
    program: Rc<Program>,
}

pub struct GpuCommand<'a> {
    pub vertices: VertexBuffer<GpuVertex>,
    pub indices: IndexBuffer<u32>,
    pub texture: &'a Texture2dMultisample,
    pub program: &'a Program,
    pub uniforms: &'a UniformBuffer,
}

impl Gpu {
    pub fn new() -> Result<Self> {
        let events_loop = glium::glutin::EventsLoop::new();
        let ctx = glium::glutin::ContextBuilder::new()
            .with_multisampling(0)
            .build_headless(
                &events_loop,
                PhysicalSize {
                    width: 0.0,
                    height: 0.0,
                },
            )?;
        let ctx = Rc::new(Headless::new(ctx)?);

        let program = Rc::new(Program::from_source(
            ctx.as_ref(),
            VERTEX_SHADER,
            FRAGMENT_SHADER,
            None,
        )?);

        Ok(Gpu { program, ctx })
    }

    pub fn default_shader(&self, width: f32, height: f32) -> Shader {
        Shader {
            id: random(),
            program: self.program.clone(),
            uniforms: UniformBuffer {
                uniforms: vec![
                    (String::from("width"), UniformValue::Float(width)),
                    (String::from("height"), UniformValue::Float(height)),
                ],
            },
        }
    }

    pub fn compile_glsl(&self, source: &str) -> Result<Rc<Program>> {
        Ok(Rc::new(Program::from_source(
            self.ctx.as_ref(),
            VERTEX_SHADER,
            source,
            None,
        )?))
    }

    pub fn build_shader(&self, program: Rc<Program>, uniforms: UniformBuffer) -> Result<Shader> {
        Ok(Shader {
            id: random(),
            program,
            uniforms,
        })
    }

    pub fn build_texture(&self, width: u32, height: u32) -> Result<Texture2dMultisample> {
        Ok(Texture2dMultisample::empty_with_format(
            self.ctx.as_ref(),
            UncompressedFloatFormat::F32F32F32F32,
            MipmapsOption::NoMipmap,
            width,
            height,
            /*samples=*/ 1,
        )?)
    }

    pub fn build_ram_texture(&self, width: u32, height: u32) -> Result<Texture2d> {
        Ok(Texture2d::empty_with_format(
            self.ctx.as_ref(),
            UncompressedFloatFormat::F32F32F32F32,
            MipmapsOption::NoMipmap,
            width,
            height,
        )?)
    }

    pub fn precompose(
        &self,
        width: u32,
        height: u32,
        elements: impl Iterator<Item = Element>,
    ) -> Result<Rc<Texture2dMultisample>> {
        let texture = self.build_texture(width, height)?;
        for (_id, batch) in &elements.group_by(|e| e.shader.id) {
            let mut batch = batch.peekable();
            let mut first = if let Some(first) = batch.peek() {
                first.shader.clone()
            } else {
                println!("This is possible??");
                continue;
            };

            // TODO: reconcile conflicts between user uniforms and the defaults
            first
                .uniforms
                .push(String::from("width"), UniformValue::Float(width as f32));
            first
                .uniforms
                .push(String::from("height"), UniformValue::Float(height as f32));

            let (indices, vertices) = self.build_buffers(batch)?;
            self.draw_to_texture(GpuCommand {
                indices,
                vertices,
                texture: &texture,
                program: first.program.as_ref(),
                uniforms: &first.uniforms,
            })?;
        }
        Ok(Rc::new(texture))
    }

    pub fn read_to_ram(&self, texture: &Texture2dMultisample) -> Result<RawImage2d<u8>> {
        let (width, height) = texture.dimensions();
        let target = self.build_ram_texture(width, height)?;
        texture.as_surface().blit_color(
            &glium::Rect {
                left: 0,
                bottom: 0,
                width,
                height,
            },
            &target.as_surface(),
            &glium::BlitTarget {
                left: 0,
                bottom: 0,
                width: width as i32,
                height: height as i32,
            },
            MagnifySamplerFilter::Linear,
        );

        Ok(target.read())
    }

    fn draw_to_texture(&self, cmd: GpuCommand) -> Result<()> {
        Ok(cmd.texture.as_surface().draw(
            &cmd.vertices,
            &cmd.indices,
            cmd.program,
            cmd.uniforms,
            &DrawParameters {
                blend: Blend {
                    color: BlendingFunction::Addition {
                        source: LinearBlendingFactor::SourceAlpha,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    },
                    alpha: BlendingFunction::Addition {
                        source: LinearBlendingFactor::One,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    },
                    constant_value: (0.0, 0.0, 0.0, 0.0),
                },
                line_width: Some(1.0),
                multisampling: false,
                dithering: false,
                smooth: None,
                ..Default::default()
            },
        )?)
    }

    fn build_buffers(
        &self,
        mut elements: impl Iterator<Item = Element>,
    ) -> Result<(IndexBuffer<u32>, VertexBuffer<GpuVertex>)> {
        let (_, vertices, indices) = elements
            .try_fold::<_, _, Result<(u32, Vec<GpuVertex>, Vec<u32>)>>(
                (0, vec![], vec![]),
                |(idx, mut vertices, mut indices), element| {
                    let (mut new_vertices, new_indices) =
                        raster_path(element.path, element.raster_method, element.color)?;
                    vertices.append(&mut new_vertices);
                    indices.extend(new_indices.into_iter().map(|i| i + idx));
                    Ok((vertices.len() as u32, vertices, indices))
                },
            )?;

        let vertex_buffer = VertexBuffer::new(self.ctx.as_ref(), vertices.as_slice())?;
        let index_buffer = IndexBuffer::new(
            self.ctx.as_ref(),
            PrimitiveType::TrianglesList,
            indices.as_slice(),
        )?;

        Ok((index_buffer, vertex_buffer))
    }
}
