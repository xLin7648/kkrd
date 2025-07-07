use image::DynamicImage;

use crate::*;

pub const FRAG_SHADER_PREFIX: &str = include_str!("shaders/frag-shader-prefix.wgsl");

pub const CAMERA_BIND_GROUP_PREFIX: &str = include_str!("shaders/camera-bind-group.wgsl");

pub const SHADER_POST_PROCESSING_VERTEX: &str = include_str!("shaders/post_processing_vertex.wgsl");

pub const COPY_SHADER_SRC: &str = include_str!("shaders/copy.wgsl");

pub fn sprite_shader_from_fragment(source: &str) -> String {
    format!("{}{}{}", CAMERA_BIND_GROUP_PREFIX, FRAG_SHADER_PREFIX, source)
}

pub fn post_process_shader_from_fragment(source: &str) -> String {
    format!(
        "{}{}{}",
        CAMERA_BIND_GROUP_PREFIX, SHADER_POST_PROCESSING_VERTEX, source
    )
}

pub fn create_engine_post_processing_shader(
    shaders: &mut ShaderMap,
    name: &str,
    shader_str: &str,
) -> Shader {
    let full_shader = post_process_shader_from_fragment(shader_str);

    let shader_id = create_shader(shaders, name, &full_shader, HashMap::new())
        .expect("Failed to create shader");

    shaders.get(shader_id).expect("Shader not found").clone()
}

pub fn load_texture_from_engine_bytes(
    context: &GraphicsContext,
    name: &str,
    bytes: &[u8],
    textures: &mut TextureMap,
    address_mode: wgpu::AddressMode,
) {
    let img = image::load_from_memory(bytes).expect("must be valid image");
    let texture = Texture::from_image_ex(
        &context.device,
        &context.queue,
        &img,
        Some(name),
        false,
        address_mode,
    )
    .unwrap();

    load_texture_with_image(context, name, img, texture, textures);
}

/// Loads a pre-created `Texture` with an associated `DynamicImage`
/// into the asset store.
///
/// Useful for when the user wants to create a Texture on their own,
/// e.g. by using a more exotic format and way of loading of the image.
pub fn load_texture_with_image(
    context: &GraphicsContext,
    name: &str,
    img: DynamicImage,
    texture: Texture,
    textures: &mut TextureMap,
) {
    let handle = texture_path(name);
    
    let bind_group = context.device.simple_bind_group(
        Some(&format!("{}_bind_group", name)),
        &texture,
        &context.texture_layout,
    );

    ASSETS.write().insert_handle(name, handle);
    ASSETS
        .write()
        .texture_image_map
        .lock()
        .insert(handle, Arc::new(img.to_rgba8()));
    textures.insert(handle, BindableTexture { bind_group, texture });
}