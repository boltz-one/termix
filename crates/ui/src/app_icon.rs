use std::sync::Arc;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn window_icon() -> Option<Arc<image::RgbaImage>> {
    let bytes = include_bytes!("../assets/app-icon/icon-256.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(Arc::new(img))
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
pub fn window_icon() -> Option<Arc<image::RgbaImage>> {
    None
}
