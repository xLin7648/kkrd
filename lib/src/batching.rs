use wgpu::TextureView;

use crate::*;

pub fn run_batched_render_passes(
    context: &mut WgpuRenderer,
    sample_count: Msaa,
    clear_color: Color,
    sprite_shader_id: ShaderId,
    error_shader_id: ShaderId,
    default_surface: &TextureView
) {
    let mut is_first = true;

    let queues = consume_render_queues();

    // let render_passes = {
    //     span_with_timing!("collect_render_passes");
    //
    //     let mut render_passes =
    //         HashMap::<MeshGroupKey, Vec<RenderPassData>>::new();
    //
    //     for (key, queue) in queues.into_iter() {
    //         render_passes.entry(key).or_default().push(RenderPassData {
    //             z_index: key.z_index,
    //             blend_mode: key.blend_mode,
    //             shader: key.shader,
    //             render_target: key.render_target,
    //             texture: key.texture_id,
    //             data: queue.into(),
    //         });
    //     }
    //
    //     render_passes
    // };

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
            is_first
        );

        is_first = false;
    }

    if is_first {
        render_meshes(
            context,
            sample_count,
            clear_color,
            MeshDrawData {
                blend_mode: BlendMode::Alpha,
                texture: TextureHandle::from_path("1px"),
                shader: ShaderInstanceId::default(),
                render_target: RenderTargetId::default(),
                data: Default::default(),
            },
            sprite_shader_id,
            error_shader_id,
            default_surface,
            is_first
        );

        // MeshGroupKey {
        //     z_index: 0,
        //     blend_mode: BlendMode::Alpha,
        //     texture_id: TextureHandle::from_path("1px"),
        //     shader: None,
        //     render_target: None,
        // },
        // RenderPassData {
        //     z_index: 0,
        //     blend_mode: BlendMode::Alpha,
        //     texture: TextureHandle::from_path("1px"),
        //     shader: None,
        //     render_target: None,
        //     data: SmallVec::new(),
        // },
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
    let pipeline_name = ensure_pipeline_exists(context, &pass_data, sprite_shader_id, sample_count.into());

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
    // let render_targets = c.render_targets.borrow();

    let mut encoder = context.context.device.simple_encoder("Mesh Render Encoder");

    {
        let clear_color = color_to_clear_op(
            if is_first { 
                Some(clear_color) 
            } else { 
                None 
            }
        );

        let surface = if sample_count != Msaa::Off {
            &context.msaa_texture.0
        } else {
            default_surface
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Mesh Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: clear_color,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_stencil_attachment(
                context.enable_z_buffer,
                &context.depth_texture.view,
                is_first,
            ),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mesh_pipeline = context
            .user_pipelines
            .get(&pipeline_name)
            .map(RenderPipeline::User)
            .or_else(|| context.pipelines.get(&pipeline_name).map(RenderPipeline::Wgpu))
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
                .set_index_buffer(context.index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
        }

        /* let tex_bind_group = match tex_handle {
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
        }; */

        let tex_bind_group = &textures
            .get(&tex_handle)
            .unwrap_or_else(|| {
                textures
                    .get(&texture_id("error"))
                    .expect("error texture must exist")
            })
            .bind_group;

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

    context.context.queue.submit(std::iter::once(encoder.finish()));
}
