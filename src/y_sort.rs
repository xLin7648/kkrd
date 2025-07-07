use crate::*;

static Y_SORT_FLAGS: Lazy<RwLock<HashMap<i32, bool>>> =
    Lazy::new(|| RwLock::new(HashMap::default()));

pub fn set_y_sort(z_index: i32, value: bool) {
    Y_SORT_FLAGS.write().insert(z_index, value);
}

pub fn get_y_sort(z_index: i32) -> bool {
    *Y_SORT_FLAGS.read().get(&z_index).unwrap_or(&false)
}
