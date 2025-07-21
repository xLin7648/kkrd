use std::error;

use crate::*;

static SHADER_UNIFORM_TABLE: Lazy<RwLock<ShaderUniformTable>> =
    Lazy::new(|| RwLock::new(ShaderUniformTable::default()));

#[derive(Default)]
pub struct ShaderUniformTable {
    instances: Vec<ShaderInstance>,
}

pub fn clear_shader_uniform_table() {
    SHADER_UNIFORM_TABLE.write().instances.clear();
}

pub fn get_shader_instance(
    id: ShaderInstanceId,
) -> MappedRwLockReadGuard<'static, ShaderInstance> {
    RwLockReadGuard::map(SHADER_UNIFORM_TABLE.read(), |x| {
        &x.instances[id.0 as usize - 1]
    })
}

pub fn set_uniform(name: impl Into<String>, value: Uniform) {
    let instance_id = CURRENT_SHADER_INSTANCE_ID.load(Ordering::SeqCst);

    if instance_id > 0 {
        let mut table = SHADER_UNIFORM_TABLE.write();

        if let Some(instance) = table.instances.get(instance_id as usize - 1) {
            let mut new_instance = instance.clone();
            new_instance.uniforms.insert(name.into(), value);

            // TIME Uniform
            {
                let timer = get_timer().read().clone();

                let cur_time = timer.get_time();

                let times: [OrderedFloat<f32>; 4] = [
                    OrderedFloat::<f32>(timer.get_time()),
                    OrderedFloat::<f32>(cur_time.sin()),
                    OrderedFloat::<f32>(cur_time.cos()),
                    OrderedFloat::<f32>(timer.get_delta_time())
                ];

                new_instance.uniforms.insert("time".to_owned(), Uniform::Vec4(times));
            }

            table.instances.push(new_instance);

            CURRENT_SHADER_INSTANCE_ID
                .store(table.instances.len() as u32, Ordering::SeqCst);
        } else {
            panic!("Current shader instance id is invalid.");
            // This is likely a bug, \
            //     please report an issue on https://github.com/darthdeus/comfy/issues with \
            //     some information on what you did."
        }
    } else {
        // error!("Trying to set a uniform with no shader active");
    }
}

static CURRENT_SHADER_INSTANCE_ID: AtomicU32 = AtomicU32::new(0);

pub fn use_shader(shader_id: ShaderId) {
    let mut table = SHADER_UNIFORM_TABLE.write();

    table
        .instances
        .push(ShaderInstance { id: shader_id, uniforms: Default::default() });

    CURRENT_SHADER_INSTANCE_ID
        .store(table.instances.len() as u32, Ordering::SeqCst);
}

pub fn use_default_shader() {
    CURRENT_SHADER_INSTANCE_ID.store(0, Ordering::SeqCst);
}

pub fn get_current_shader() -> ShaderInstanceId {
    ShaderInstanceId(CURRENT_SHADER_INSTANCE_ID.load(Ordering::SeqCst))
}

static SHADER_IDS: AtomicU64 = AtomicU64::new(0);

pub fn gen_shader_id() -> ShaderId {
    let id = SHADER_IDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    ShaderId(id)
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShaderInstanceId(pub u32);

static RENDER_QUEUES: Lazy<RwLock<RenderQueues>> =
    Lazy::new(|| RwLock::new(RenderQueues::default()));

pub type RenderQueue = Vec<Mesh>;

#[derive(Default)]
struct RenderQueues {
    data: BTreeMap<MeshGroupKey, RenderQueue>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshGroupKey {
    pub z_index: i32,
    pub blend_mode: BlendMode,
    pub texture_id: TextureHandle,
    pub shader: ShaderInstanceId,
    pub render_target: RenderTargetId,
}

pub fn consume_render_queues() -> BTreeMap<MeshGroupKey, RenderQueue> {
    let mut queues = RENDER_QUEUES.write();
    std::mem::take(&mut queues.data)
}

pub fn queue_mesh_draw(mesh: Mesh, blend_mode: BlendMode) {
    let shader = get_current_shader();
    let render_target = get_current_render_target();

    RENDER_QUEUES
        .write()
        .data
        .entry(MeshGroupKey {
            z_index: mesh.z_index,
            blend_mode,
            texture_id: mesh
                .texture
                .unwrap_or_else(|| TextureHandle::from_path("1px")),
            shader,
            render_target,
        })
        .or_default()
        .push(mesh);
}
