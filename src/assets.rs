use crate::*;

pub static ASSETS: Lazy<AtomicRefCell<Assets>> =
    Lazy::new(|| AtomicRefCell::new(Assets::new()));

pub fn texture_id_safe(id: &str) -> Option<TextureHandle> {
    ASSETS.borrow().textures.get(id).copied()
}

pub struct Assets {
    pub textures: HashMap<String, TextureHandle>,
    
    pub texture_image_map:
        Arc<Mutex<HashMap<TextureHandle, Arc<image::RgbaImage>>>>,
}

impl Assets {
    pub fn new() -> Self {
        let image_map = Arc::new(Mutex::new(HashMap::new()));

        Self {
            textures: Default::default(),
            texture_image_map: image_map,
        }
    }

    pub fn image_size(handle: TextureHandle) -> ImageSizeResult {
        let assets = ASSETS.borrow();
        let image_map = assets.texture_image_map.lock();
    
    
        if let Some(image) = image_map.get(&handle) {
            ImageSizeResult::Loaded(uvec2(image.width(), image.height()))
        } else {
            ImageSizeResult::ImageNotFound
        }
    }

    pub fn insert_handle(&mut self, name: &str, handle: TextureHandle) {
        self.textures.insert(name.to_string(), handle);
    }
}

pub fn texture_id(id: &str) -> TextureHandle {
    if id == "1px" {
        texture_id_safe("1px").expect("1px must be loaded")
    } else {
        texture_id_safe(id).unwrap_or_else(|| {
            if id == "error" {
                for key in ASSETS.borrow().textures.keys().sorted() {
                    println!("{key}");
                }

                panic!("Failed to load error texture with ID = '{}'", id)
            }

            texture_id("error")
        })
    }
}

// TODO: rename to something like "unchecked_id"
pub fn texture_path(path: &str) -> TextureHandle {
    TextureHandle::from_path(path)
}

#[derive(Copy, Clone, Debug)]
pub enum ImageSizeResult {
    ImageNotFound,
    LoadingInProgress,
    Loaded(UVec2),
}

