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

#[derive(Serialize, Deserialize, Debug)]
pub struct WindowSettings {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
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
        let cache_dir = if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(local_appdata).join("fastview")
        } else {
            let mut exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
            exe_path.pop();
            exe_path
        };
        
        let db_path = cache_dir.join("fastview_cache");
        std::fs::create_dir_all(&cache_dir).ok();
        
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

    pub fn get_window_settings(&self) -> Option<WindowSettings> {
        let result = self.db.get("window_settings").ok()??;
        bincode::deserialize(&result).ok()
    }

    pub fn set_window_settings(&self, settings: &WindowSettings) {
        if let Ok(data) = bincode::serialize(settings) {
            let _ = self.db.insert("window_settings", data);
            let _ = self.db.flush();
        }
    }
}
