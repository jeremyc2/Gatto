//! Reusable native outlines for short, positioned pieces of shaped text.
use super::*;
use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::DefaultHasher},
    hash::Hasher,
};

type Key = (u64, u32, [u8; 3]);
struct Entry {
    source: Path,
    mesh: Arc<NativePaths>,
}
#[derive(Default)]
struct Cache {
    entries: HashMap<Key, Entry>,
    bytes: usize,
}
const BUDGET: usize = 4 * 1024 * 1024;
const MAX_ENTRIES: usize = 512;
thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
}

pub(super) fn get(path: &Path, font_size: f32, scale: f32, color: [u8; 3]) -> Arc<NativePaths> {
    let mut hash = DefaultHasher::new();
    for verb in path.verbs() {
        hash.write_u8(*verb as u8);
    }
    for point in path.points() {
        hash.write_u32(point.x.to_bits());
        hash.write_u32(point.y.to_bits());
    }
    let key = (
        hash.finish(),
        (font_size / 12.).max(2. * scale).to_bits(),
        color,
    );
    CACHE
        .with(|cache| {
            let cache = cache.borrow();
            cache
                .entries
                .get(&key)
                .filter(|entry| entry.source == *path)
                .map(|entry| entry.mesh.clone())
        })
        .unwrap_or_else(|| {
            let mesh = Arc::new(outlined_text(path, font_size, scale, color));
            let bytes = std::mem::size_of_val(path.points())
                + std::mem::size_of_val(path.verbs())
                + mesh
                    .iter()
                    .map(|(path, _, _)| std::mem::size_of_val(path.vertices.as_slice()))
                    .sum::<usize>();
            if bytes <= BUDGET {
                CACHE.with(|cache| {
                    let mut cache = cache.borrow_mut();
                    if cache.entries.len() >= MAX_ENTRIES || cache.bytes + bytes > BUDGET {
                        cache.entries.clear();
                        cache.bytes = 0;
                    }
                    cache.entries.insert(
                        key,
                        Entry {
                            source: path.clone(),
                            mesh: mesh.clone(),
                        },
                    );
                    cache.bytes += bytes;
                });
            }
            mesh
        })
}
