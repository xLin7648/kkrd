use std::{collections::HashSet, hash::Hash};

use wgpu::{
    AddressMode, BindingResource, BufferBinding, CommandEncoderDescriptor, IndexFormat, LoadOp,
    Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SamplerDescriptor, StoreOp, TextureView, TextureViewDescriptor,
};

use crate::*;

pub fn run_batched_render_passes(
    renderer: &mut WgpuRenderer,
    sample_count: Msaa,
    sprite_shader_id: ShaderId,
    error_shader_id: ShaderId,
) {
    let queues = consume_render_queues();

    for (key, mut meshes) in queues.into_iter().sorted_by_key(|(k, _)| k.z_index) {
        if get_y_sort(key.z_index) {
            meshes.sort_by_key(|mesh| OrderedFloat::<f32>(-(mesh.origin.y + mesh.y_sort_offset)));
        }

        render_meshes(
            renderer,
            sample_count,
            MeshDrawData {
                blend_mode: key.blend_mode,
                texture: key.texture_id,
                shader: key.shader,
                render_target: key.render_target,
                data: meshes,
            },
            sprite_shader_id,
            error_shader_id,
        );
    }
}

pub fn render_meshes(
    renderer: &mut WgpuRenderer,
    sample_count: Msaa,
    pass_data: MeshDrawData,
    sprite_shader_id: ShaderId,
    _error_shader_id: ShaderId,
) {
    // 1. 准备管线
    let pipeline_name =
        ensure_pipeline_exists(renderer, &pass_data, sprite_shader_id, sample_count.into());

    // 2. 合并所有顶点和索引
    let mut all_vertices = Vec::<SpriteVertex>::new();
    let mut all_indices = Vec::<u32>::new();
    for mesh in pass_data.data {
        let offset = all_vertices.len() as u32;
        all_vertices.extend(mesh.vertices);
        all_indices.extend(mesh.indices.iter().map(|idx| idx + offset));
    }

    // 3. 上传顶点 / 索引
    renderer.vertex_buffer.ensure_size_and_copy(
        &renderer.context.device,
        &renderer.context.queue,
        bytemuck::cast_slice(&all_vertices),
    );
    renderer.index_buffer.ensure_size_and_copy(
        &renderer.context.device,
        &renderer.context.queue,
        bytemuck::cast_slice(&all_indices),
    );

    // 4. 选择渲染目标
    let rts = renderer.render_targets.lock();
    let rt = &rts.get(&pass_data.render_target).unwrap().lock();

    let (color_view, depth_view, resolve_target) = if sample_count != Msaa::Off {
        (&rt.msaa_view, &rt.msaa_depth_view, Some(&rt.resolve_view))
    } else {
        (&rt.resolve_view, &rt.msaa_depth_view, None)
    };

    // 5. 创建 encoder & render pass
    let mut encoder = renderer
        .context
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Mesh Render Encoder"),
        });

    {
        let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Mesh Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color_view, // MSAA 视图
                resolve_target,   // 自动 resolve 到 1-sample
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view, // 同样用 MSAA 深度
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        // 6. 设置管线与绑定组
        let mesh_pipeline = renderer
            .user_pipelines
            .get(&pipeline_name)
            .map(RenderPipeline::User)
            .or_else(|| {
                renderer
                    .pipelines
                    .get(&pipeline_name)
                    .map(RenderPipeline::Wgpu)
            })
            .expect("pipeline ensured");

        match &mesh_pipeline {
            RenderPipeline::User(p) => rp.set_pipeline(&p.pipeline),
            RenderPipeline::Wgpu(p) => rp.set_pipeline(p),
        }

        rp.set_vertex_buffer(0, renderer.vertex_buffer.buffer.slice(..));
        if !all_indices.is_empty() {
            rp.set_index_buffer(renderer.index_buffer.buffer.slice(..), IndexFormat::Uint32);
        }

        // 7. 纹理绑定
        let textures = renderer.textures.lock();
        let tex_bind_group = match pass_data.texture {
            TextureHandle::RenderTarget(rt_id) => {
                &if let Some(rt) = rts.get(&rt_id) {
                    rt.lock()
                } else if let Some(rt) = rts.get(&RenderTargetId(0)) {
                    rt.lock()
                } else {
                    panic!("No Default RendererTarget");
                }
                .blit_bind_group
            }
            TextureHandle::Path(id) | TextureHandle::Raw(id) => {
                &textures
                    .get(&pass_data.texture)
                    .unwrap_or_else(|| textures.get(&texture_id("error")).unwrap())
                    .bind_group
            }
        };
        rp.set_bind_group(0, tex_bind_group, &[]);
        rp.set_bind_group(1, renderer.camera_bind_group.as_ref(), &[]);

        if let RenderPipeline::User(p) = &mesh_pipeline {
            rp.set_bind_group(2, &p.bind_group, &[]);
        }

        // 8. 绘制
        if all_indices.is_empty() {
            rp.draw(0..all_vertices.len() as u32, 0..1);
        } else {
            rp.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
        }
    }

    renderer
        .context
        .queue
        .submit(std::iter::once(encoder.finish()));
}
