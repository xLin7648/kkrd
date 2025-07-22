use wgpu::{
    IndexFormat, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TextureView,
};

use crate::*;

pub fn run_batched_render_passes(
    context: &mut WgpuRenderer,
    sample_count: Msaa,
    clear_color: Color,
    sprite_shader_id: ShaderId,
    error_shader_id: ShaderId,
    default_surface: &TextureView,
) {
    let mut is_first = true;

    let queues = consume_render_queues();

    for (key, mut meshes) in queues.into_iter().sorted_by_key(|(k, _)| k.z_index) {
        // TODO: add this back later
        if get_y_sort(key.z_index) {
            meshes.sort_by_key(|mesh| OrderedFloat::<f32>(-(mesh.origin.y + mesh.y_sort_offset)));
        }

        render_meshes(
            context,
            sample_count,
            clear_color,
            MeshDrawData {
                blend_mode: key.blend_mode,
                texture: key.texture_id,
                shader: key.shader,
                render_target: key.render_target,
                data: meshes,
            },
            sprite_shader_id,
            error_shader_id,
            default_surface,
            is_first,
        );

        is_first = false;
    }
}

pub fn render_meshes(
    context: &mut WgpuRenderer,
    sample_count: Msaa,
    clear_color: Color,
    pass_data: MeshDrawData,
    sprite_shader_id: ShaderId,
    _error_shader_id: ShaderId,
    default_surface: &TextureView,
    is_first: bool,
) {
    let pipeline_name = if pass_data.render_target.0 > 0 {
        ensure_pipeline_exists(context, &pass_data, sprite_shader_id, 1)
    } else {
        if sample_count != Msaa::Off {
            ensure_pipeline_exists(context, &pass_data, sprite_shader_id, sample_count.into())
        } else {
            ensure_pipeline_exists(context, &pass_data, sprite_shader_id, 1)
        }
    };

    let tex_handle = pass_data.texture;

    let mut all_vertices: Vec<SpriteVertex> = vec![];
    let mut all_indices = vec![];

    for mesh in pass_data.data.into_iter() {
        let offset = all_vertices.len() as u32;
        all_vertices.extend(&mesh.vertices);
        all_indices.extend(mesh.indices.iter().map(|x| *x + offset));
    }

    context.vertex_buffer.ensure_size_and_copy(
        &context.context.device,
        &context.context.queue,
        bytemuck::cast_slice(all_vertices.as_slice()),
    );

    context.index_buffer.ensure_size_and_copy(
        &context.context.device,
        &context.context.queue,
        bytemuck::cast_slice(all_indices.as_slice()),
    );

    let textures = context.textures.lock();
    let render_targets = context.render_targets.lock();

    let mut encoder = context.context.device.simple_encoder("Mesh Render Encoder");

    {
        let clear_color = color_to_clear_op(if is_first { Some(clear_color) } else { None });

        let target_view = if pass_data.render_target.0 > 0 {
            &render_targets
                .get(&pass_data.render_target)
                .expect("user render target must exist when used")
                .view
        } else {
            if sample_count != Msaa::Off {
                &context.msaa_texture
            } else {
                default_surface
            }
        };

        // TODO: 如果使用RT，那么深度附件的尺寸需和RT尺寸保持一致
        // 否则调整窗口大小 depth_texture 尺寸一更改就会panic
        let depth_view = if pass_data.render_target.0 > 0 {
            &context.depth_texture // 需要改这里
        } else {
            if sample_count != Msaa::Off {
                &context.msaa_depth_texture
            } else {
                &context.depth_texture
            }
        };

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Mesh Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: Operations {
                    load: clear_color,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_stencil_attachment(
                context.enable_z_buffer,
                depth_view,
                is_first,
            ),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mesh_pipeline = context
            .user_pipelines
            .get(&pipeline_name)
            .map(RenderPipeline::User)
            .or_else(|| {
                context
                    .pipelines
                    .get(&pipeline_name)
                    .map(RenderPipeline::Wgpu)
            })
            .expect("ensured pipeline must exist within the same frame");

        match &mesh_pipeline {
            RenderPipeline::User(pipeline) => {
                render_pass.set_pipeline(&pipeline.pipeline);
            }
            RenderPipeline::Wgpu(pipeline) => {
                render_pass.set_pipeline(pipeline);
            }
        }

        render_pass.set_vertex_buffer(0, context.vertex_buffer.buffer.slice(..));

        if !all_indices.is_empty() {
            render_pass
                .set_index_buffer(context.index_buffer.buffer.slice(..), IndexFormat::Uint32);
        }

        let tex_bind_group = match tex_handle {
            TextureHandle::RenderTarget(render_target_id) => {
                &render_targets.get(&render_target_id).unwrap().bind_group
            }
            _ => {
                &textures
                    .get(&tex_handle)
                    .unwrap_or_else(|| {
                        textures
                            .get(&texture_id("error"))
                            .expect("error texture must exist")
                    })
                    .bind_group
            }
        };

        render_pass.set_bind_group(0, tex_bind_group, &[]);
        render_pass.set_bind_group(1, context.camera_bind_group.as_ref(), &[]);

        match &mesh_pipeline {
            RenderPipeline::User(pipeline) => {
                render_pass.set_bind_group(2, &pipeline.bind_group, &[]);
            }
            RenderPipeline::Wgpu(_) => {}
        }

        if all_indices.is_empty() {
            render_pass.draw(0..all_vertices.len() as u32, 0..1);
        } else {
            render_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
        }
    }

    context
        .context
        .queue
        .submit(std::iter::once(encoder.finish()));
}
