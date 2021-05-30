use std::path::Path;

use crate::renderer::{Renderer, Texture};

#[non_exhaustive]
pub struct ResourceManager;

impl ResourceManager {
    pub fn new() -> Self { Self }

    pub fn load_texture_from_file(&self, renderer: &Renderer, path: &Path) -> Option<Texture> {
        println!("Loading {}", path.to_string_lossy());
        let image = image::io::Reader::open(path)
            .ok()?
            .decode()
            .ok()?
            .to_rgba8();
        Some(Texture::create_texture(
            renderer,
            &image,
            Some(&path.to_string_lossy()),
        ))
    }
}
