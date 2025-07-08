use wgpu::TextureView;

use crate::*;

pub fn run_batched_render_passes(
    c: &mut WgpuRenderer,
    clear_color: Color,
    sprite_shader_id: ShaderId,
    error_shader_id: ShaderId,
    default_surface_view: &TextureView
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

    for (key, mut meshes) in
        queues.into_iter().sorted_by_key(|(k, _)| k.z_index)
    {
        // TODO: add this back later
        if get_y_sort(key.z_index) {
            meshes.sort_by_key(|mesh| {
                OrderedFloat::<f32>(-(mesh.origin.y + mesh.y_sort_offset))
            });
        }

        render_meshes(
            c,
            is_first,
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
            default_surface_view
        );


        is_first = false;
    }

    if is_first {
        render_meshes(
            c,
            is_first,
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
            default_surface_view
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
    c: &mut WgpuRenderer,
    is_first: bool,
    clear_color: Color,
    pass_data: MeshDrawData,
    sprite_shader_id: ShaderId,
    _error_shader_id: ShaderId,
    default_surface_view: &TextureView
) {
    let pipeline_name = ensure_pipeline_exists(c, &pass_data, sprite_shader_id);

    let tex_handle = pass_data.texture;

    let mut all_vertices: Vec<SpriteVertex> = vec![];
    let mut all_indices = vec![];

    for mesh in pass_data.data.into_iter() {
        let offset = all_vertices.len() as u32;
        all_vertices.extend(&mesh.vertices);
        all_indices.extend(mesh.indices.iter().map(|x| *x + offset));
    }

    c.vertex_buffer.ensure_size_and_copy(
        &c.context.device,
        &c.context.queue,
        bytemuck::cast_slice(all_vertices.as_slice()),
    );

    c.index_buffer.ensure_size_and_copy(
        &c.context.device,
        &c.context.queue,
        bytemuck::cast_slice(all_indices.as_slice()),
    );

    let textures = c.textures.lock();
    // let render_targets = c.render_targets.borrow();

    let mut encoder = c.context.device.simple_encoder("Mesh Render Encoder");

    {
        let clear_color = if is_first { Some(clear_color) } else { None };

        /* let target_view = if pass_data.render_target.0 > 0 {
            &render_targets
                .get(&pass_data.render_target)
                .expect("user render target must exist when used")
                .view
        } else if c.post_processing_effects.borrow().iter().any(|x| x.enabled) {
            &c.first_pass_texture.texture.view
        } else {
            c.msaa_texture.as_mut().unwrap()
        }; */

        let surface= if game_config().sample_count == Msaa::Off { 
            &default_surface_view
        } else { 
            &c.msaa_texture
        };

        let mut render_pass =
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Mesh Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_to_clear_op(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_stencil_attachment(
                    c.enable_z_buffer,
                    &c.depth_texture.view,
                    is_first,
                ),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        let mesh_pipeline = c
            .user_pipelines
            .get(&pipeline_name)
            .map(RenderPipeline::User)
            .or_else(|| {
                c.pipelines.get(&pipeline_name).map(RenderPipeline::Wgpu)
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

        render_pass.set_vertex_buffer(0, c.vertex_buffer.buffer.slice(..));

        if !all_indices.is_empty() {
            render_pass.set_index_buffer(
                c.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
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
        render_pass.set_bind_group(1, c.camera_bind_group.as_ref(), &[]);

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

    c.context.queue.submit(std::iter::once(encoder.finish()));
    
}