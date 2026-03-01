use crate::metadata::{apply_orientation, ImageMetadata};
use image::{DynamicImage, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum FileItem {
    Image(PathBuf),
    Directory(PathBuf),
}

pub struct ImageLoader {
    folder_path: PathBuf,
    items: Vec<FileItem>,
    image_files: Vec<PathBuf>,
    current_index: usize,
}

impl ImageLoader {
    pub fn new(mut folder_path: PathBuf) -> Self {
        // Canonicalize path to ensure reliable matching
        folder_path = fs::canonicalize(&folder_path).unwrap_or(folder_path);

        let mut slf = ImageLoader {
            folder_path,
            items: Vec::new(),
            image_files: Vec::new(),
            current_index: 0,
        };
        slf.refresh();
        slf
    }

    pub fn refresh(&mut self) {
        self.items.clear();
        self.image_files.clear();

        if let Ok(entries) = fs::read_dir(&self.folder_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();

                if path.is_dir() {
                    self.items.push(FileItem::Directory(path));
                } else if is_image_file(&path) {
                    self.items.push(FileItem::Image(path.clone()));
                    self.image_files.push(path);
                }
            }
        }

        self.items.sort_by(|a, b| match (a, b) {
            (FileItem::Directory(_), FileItem::Image(_)) => std::cmp::Ordering::Less,
            (FileItem::Image(_), FileItem::Directory(_)) => std::cmp::Ordering::Greater,
            (FileItem::Directory(pa), FileItem::Directory(pb)) => pa.cmp(pb),
            (FileItem::Image(pa), FileItem::Image(pb)) => {
                let meta_a = fs::metadata(pa).ok().and_then(|m| m.modified().ok());
                let meta_b = fs::metadata(pb).ok().and_then(|m| m.modified().ok());
                match (meta_a, meta_b) {
                    (Some(ta), Some(tb)) => tb.cmp(&ta),
                    _ => pa.cmp(pb),
                }
            }
        });

        self.image_files.sort_by(|a, b| {
            let meta_a = fs::metadata(a).ok().and_then(|m| m.modified().ok());
            let meta_b = fs::metadata(b).ok().and_then(|m| m.modified().ok());
            match (meta_a, meta_b) {
                (Some(ta), Some(tb)) => tb.cmp(&ta),
                _ => a.cmp(b),
            }
        });
        self.current_index = 0;
    }

    pub fn set_path(&mut self, mut new_path: PathBuf) {
        new_path = fs::canonicalize(&new_path).unwrap_or(new_path);
        self.folder_path = new_path;
        self.refresh();
    }

    pub fn get_path(&self) -> &Path {
        &self.folder_path
    }

    pub fn get_items(&self) -> &[FileItem] {
        &self.items
    }

    pub fn get_image_count(&self) -> usize {
        self.image_files.len()
    }

    pub fn get_current_index(&self) -> usize {
        self.current_index
    }

    pub fn get_current_path(&self) -> Option<&PathBuf> {
        if self.image_files.is_empty() {
            return None;
        }
        Some(&self.image_files[self.current_index])
    }

    pub fn load_current_image(&self) -> Option<RgbaImage> {
        if self.image_files.is_empty() {
            return None;
        }

        let path = &self.image_files[self.current_index];

        if let Some(img) = Self::load_dynamic_image_path_with_metadata(path) {
            Some(img.to_rgba8())
        } else {
            None
        }
    }

    pub fn load_dynamic_image_path_with_metadata(path: &Path) -> Option<DynamicImage> {
        let metadata = ImageMetadata::from_path(path);
        let img = image::open(path).ok()?;

        if metadata.orientation.needs_rotation() {
            Some(apply_orientation(&img, metadata.orientation))
        } else {
            Some(img)
        }
    }

    pub fn get_current_metadata(&self) -> Option<ImageMetadata> {
        if self.image_files.is_empty() {
            return None;
        }
        let path = &self.image_files[self.current_index];
        Some(ImageMetadata::from_path(path))
    }

    pub fn load_image_path(path: &Path) -> Option<RgbaImage> {
        if let Ok(img) = image::open(path) {
            Some(img.to_rgba8())
        } else {
            None
        }
    }

    pub fn load_dynamic_image_path(path: &Path) -> Option<DynamicImage> {
        image::open(path).ok()
    }

    pub fn next_image(&mut self) -> Option<RgbaImage> {
        if self.image_files.is_empty() {
            return None;
        }

        self.current_index = (self.current_index + 1) % self.image_files.len();
        self.load_current_image()
    }

    pub fn prev_image(&mut self) -> Option<RgbaImage> {
        if self.image_files.is_empty() {
            return None;
        }

        if self.current_index == 0 {
            self.current_index = self.image_files.len() - 1;
        } else {
            self.current_index -= 1;
        }

        self.load_current_image()
    }

    pub fn open_image(&mut self, path: &Path) -> Option<RgbaImage> {
        // Ensure path to match is also canonicalized for reliable matching
        let target = fs::canonicalize(path).unwrap_or(path.to_path_buf());
        if let Some(pos) = self.image_files.iter().position(|p| p == &target) {
            self.current_index = pos;
            self.load_current_image()
        } else {
            None
        }
    }
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| match ext.to_string_lossy().to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" => true,
            _ => false,
        })
        .unwrap_or(false)
}
