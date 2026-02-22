use sled::{Db};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use image::RgbaImage;

#[derive(Serialize, Deserialize, Debug)]
pub struct CacheEntry {
    pub mtime: u64,
    pub size: u64,
    pub thumbnail_data: Vec<u8>,
}

#[derive(Clone)]
pub struct CacheManager {
    db: Db,
}

impl CacheManager {
    pub fn clone_db_handle(&self) -> Self {
        self.clone()
    }
    pub fn new() -> Self {
        let mut exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe_path.pop();
        let db_path = exe_path.join("fastview_cache");
        
        let db = sled::open(db_path).expect("Failed to open cache database");
        CacheManager { db }
    }

    fn get_key(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    pub fn get(&self, path: &Path) -> Option<CacheEntry> {
        let key = Self::get_key(path);
        let result = self.db.get(key).ok()??;
        bincode::deserialize(&result).ok()
    }

    pub fn set(&self, path: &Path, entry: CacheEntry) {
        let key = Self::get_key(path);
        if let Ok(data) = bincode::serialize(&entry) {
            let _ = self.db.insert(key, data);
            let _ = self.db.flush();
        }
    }

    pub fn get_thumbnail(&self, path: &Path) -> Option<RgbaImage> {
        let entry = self.get(path)?;
        image::load_from_memory(&entry.thumbnail_data).ok()?.to_rgba8().into()
    }

    pub fn set_thumbnail(&self, path: &Path, img: &RgbaImage) {
        let entry = CacheEntry {
            mtime: 0,
            size: 0,
            thumbnail_data: img.to_vec(),
        };
        self.set(path, entry);
    }
}
