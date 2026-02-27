use image::DynamicImage;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExifOrientation {
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    FlipHorizontal,
    FlipHorizontalRotate90,
    FlipHorizontalRotate180,
    FlipHorizontalRotate270,
}

impl ExifOrientation {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => ExifOrientation::Normal,
            2 => ExifOrientation::FlipHorizontal,
            3 => ExifOrientation::Rotate180,
            4 => ExifOrientation::FlipHorizontalRotate180,
            5 => ExifOrientation::FlipHorizontalRotate90,
            6 => ExifOrientation::Rotate90,
            7 => ExifOrientation::FlipHorizontalRotate270,
            8 => ExifOrientation::Rotate270,
            _ => ExifOrientation::Normal,
        }
    }

    pub fn needs_rotation(&self) -> bool {
        !matches!(self, ExifOrientation::Normal)
    }

    pub fn to_string(&self) -> String {
        match self {
            ExifOrientation::Normal => "Normal".to_string(),
            ExifOrientation::Rotate90 => "Rotate 90° CW".to_string(),
            ExifOrientation::Rotate180 => "Rotate 180°".to_string(),
            ExifOrientation::Rotate270 => "Rotate 90° CCW".to_string(),
            ExifOrientation::FlipHorizontal => "Flip Horizontal".to_string(),
            ExifOrientation::FlipHorizontalRotate90 => "Flip Horizontal, Rotate 90° CW".to_string(),
            ExifOrientation::FlipHorizontalRotate180 => "Flip Horizontal, Rotate 180°".to_string(),
            ExifOrientation::FlipHorizontalRotate270 => {
                "Flip Horizontal, Rotate 90° CCW".to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExifData {
    pub make: Option<String>,
    pub model: Option<String>,
    pub date_taken: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub focal_length: Option<String>,
    pub lens_model: Option<String>,
    pub software: Option<String>,
    pub image_size: Option<String>,
    pub orientation: ExifOrientation,
}

impl ExifData {
    pub fn to_key_values(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();

        if let Some(ref make) = self.make {
            pairs.push(("Camera make".to_string(), make.clone()));
        }
        if let Some(ref model) = self.model {
            pairs.push(("Camera model".to_string(), model.clone()));
        }
        if let Some(ref date) = self.date_taken {
            pairs.push(("Date taken".to_string(), date.clone()));
        }
        if let Some(ref size) = self.image_size {
            pairs.push(("Image size".to_string(), size.clone()));
        }

        if self.orientation != ExifOrientation::Normal {
            pairs.push(("Orientation".to_string(), self.orientation.to_string()));
        }

        if let Some(ref exp) = self.exposure_time {
            pairs.push(("Exposure".to_string(), format!("{}s", exp)));
        }
        if let Some(ref f) = self.f_number {
            pairs.push(("Aperture".to_string(), format!("f/{}", f)));
        }
        if let Some(ref iso) = self.iso {
            pairs.push(("ISO".to_string(), iso.clone()));
        }
        if let Some(ref fl) = self.focal_length {
            pairs.push(("Focal length".to_string(), format!("{}mm", fl)));
        }
        if let Some(ref lens) = self.lens_model {
            pairs.push(("Lens".to_string(), lens.clone()));
        }
        if let Some(ref software) = self.software {
            pairs.push(("Software".to_string(), software.clone()));
        }

        pairs
    }
}

pub struct ImageMetadata {
    pub orientation: ExifOrientation,
    pub prompt: Option<String>,
    pub exif: Option<ExifData>,
}

impl ImageMetadata {
    pub fn from_path(path: &Path) -> Self {
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let exif = match extension.as_str() {
            "jpg" | "jpeg" => Self::read_exif_data(path),
            _ => None,
        };

        let orientation = exif
            .as_ref()
            .map(|e| e.orientation)
            .unwrap_or(ExifOrientation::Normal);

        let prompt = match extension.as_str() {
            "png" => Self::read_png_prompt(path),
            _ => None,
        };

        ImageMetadata {
            orientation,
            prompt,
            exif,
        }
    }

    pub fn get_metadata_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // EXIF data as key-value pairs
        if let Some(ref exif) = self.exif {
            for (key, value) in exif.to_key_values() {
                lines.push(format!("{}: {}", key, value));
            }
        }

        // ComfyUI workflow detection and prompt extraction
        if let Some(ref prompt) = self.prompt {
            if prompt.trim().starts_with('{') {
                lines.push("".to_string());
                lines.push("ComfyUI prompt:".to_string());
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(prompt) {
                    if let Some(extracted) = Self::extract_comfyui_prompt(&json) {
                        lines.push(format!("\"{}\"", extracted));
                    } else {
                        lines.push(format!("\"{}\"", &prompt[..prompt.len().min(200)]));
                    }
                } else {
                    lines.push(format!("\"{}\"", &prompt[..prompt.len().min(200)]));
                }
            } else {
                lines.push("".to_string());
                lines.push("Prompt:".to_string());
                let max_chars = 80;
                let chars: Vec<char> = prompt.chars().collect();
                for chunk in chars.chunks(max_chars) {
                    let chunk_str: String = chunk.iter().collect();
                    lines.push(chunk_str);
                }
            }
        }

        if lines.is_empty() {
            lines.push("No metadata found".to_string());
        }

        lines
    }

    fn read_exif_data(path: &Path) -> Option<ExifData> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let mut reader = BufReader::new(file);

        let exif = match exif::Reader::new().read_from_container(&mut reader) {
            Ok(e) => e,
            Err(_) => return None,
        };

        let get_str = |tag: exif::Tag| -> Option<String> {
            exif.get_field(tag, exif::In::PRIMARY)
                .map(|f| f.display_value().to_string().trim().to_string())
        };

        let orientation = exif
            .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
            .map(ExifOrientation::from_u32)
            .unwrap_or(ExifOrientation::Normal);

        let image_size = exif
            .get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
            .and_then(|w| {
                exif.get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
                    .and_then(|f| f.value.get_uint(0))
                    .map(|h| format!("{}x{}", w, h))
            });

        Some(ExifData {
            make: get_str(exif::Tag::Make),
            model: get_str(exif::Tag::Model),
            date_taken: get_str(exif::Tag::DateTimeOriginal)
                .or_else(|| get_str(exif::Tag::DateTime)),
            exposure_time: get_str(exif::Tag::ExposureTime),
            f_number: get_str(exif::Tag::FNumber),
            iso: get_str(exif::Tag::PhotographicSensitivity)
                .or_else(|| get_str(exif::Tag::ISOSpeed)),
            focal_length: get_str(exif::Tag::FocalLength),
            lens_model: get_str(exif::Tag::LensModel)
                .or_else(|| get_str(exif::Tag::LensSpecification)),
            software: get_str(exif::Tag::Software),
            image_size,
            orientation,
        })
    }

    fn read_png_prompt(path: &Path) -> Option<String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut reader = BufReader::new(file);
        let mut bytes = Vec::new();
        if reader.read_to_end(&mut bytes).is_err() {
            return None;
        }

        let png = match img_parts::png::Png::from_bytes(bytes.into()) {
            Ok(p) => p,
            Err(_) => return None,
        };

        // First check for tEXt with "Prompt" key
        for chunk in png.chunks() {
            let kind = chunk.kind();
            if kind == *b"tEXt" || kind == *b"iTXt" {
                if let Ok(text) = std::str::from_utf8(chunk.contents()) {
                    if text.starts_with("Prompt\0") || text.starts_with("prompt\0") {
                        return Some(text[7..].to_string());
                    }
                }
            }
        }

        // Try to find JSON workflow and extract prompt
        for chunk in png.chunks() {
            let kind = chunk.kind();
            if kind == *b"tEXt" || kind == *b"iTXt" {
                if let Ok(text) = std::str::from_utf8(chunk.contents()) {
                    if let Some(pos) = text.find('\0') {
                        let value = &text[pos + 1..];
                        // Check if it looks like JSON
                        if value.trim().starts_with('{') {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
                                // Try to extract prompt from ComfyUI workflow
                                if let Some(prompt) = Self::extract_comfyui_prompt(&json) {
                                    return Some(prompt);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: look for any text that looks like a prompt
        for chunk in png.chunks() {
            let kind = chunk.kind();
            if kind == *b"tEXt" {
                if let Ok(text) = std::str::from_utf8(chunk.contents()) {
                    if let Some(pos) = text.find('\0') {
                        let value = &text[pos + 1..];
                        if value.len() > 10 && value.len() < 2000 {
                            let is_likely_prompt = value
                                .chars()
                                .any(|c| c == ',' || c == '(' || c == '{' || c == '[');
                            if is_likely_prompt && !value.contains("AI Generated") {
                                return Some(value.to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_comfyui_prompt(json: &serde_json::Value) -> Option<String> {
        // First, try to find nodes with "prompt" in the title and get their text field
        if let Some(nodes) = json.get("nodes").or_else(|| json.get("prompt")) {
            if let Some(nodes_array) = nodes.as_array() {
                for node in nodes_array {
                    if let Some(title) = node.get("title").and_then(|v| v.as_str()) {
                        let title_lower = title.to_lowercase();
                        if title_lower.contains("prompt") {
                            if let Some(text) = node
                                .get("widgets_values")
                                .and_then(|v| v.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|v| v.as_str())
                            {
                                if !text.is_empty() && text.len() > 5 {
                                    return Some(text.to_string());
                                }
                            }
                            // Also check "text" field directly
                            if let Some(text) = node.get("text").and_then(|v| v.as_str()) {
                                if !text.is_empty() && text.len() > 5 {
                                    return Some(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Try "prompt" field directly at root level
        if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
            return Some(prompt.to_string());
        }

        // Try to find any string that looks like a prompt (contains common SD keywords)
        fn find_prompt_in_value(value: &serde_json::Value) -> Option<String> {
            match value {
                serde_json::Value::String(s) => {
                    let lower = s.to_lowercase();
                    if (lower.contains("masterpiece")
                        || lower.contains("best quality")
                        || lower.contains("8k")
                        || lower.contains("ultra detailed")
                        || lower.contains("photorealistic")
                        || lower.contains("render"))
                        && s.len() > 20
                    {
                        return Some(s.clone());
                    }
                    None
                }
                serde_json::Value::Object(map) => {
                    // Check common prompt field names
                    for key in &["prompt", "text", "description", "positive"] {
                        if let Some(v) = map.get(*key) {
                            if let Some(s) = find_prompt_in_value(v) {
                                return Some(s);
                            }
                        }
                    }
                    // Recursively search all values
                    for v in map.values() {
                        if let Some(s) = find_prompt_in_value(v) {
                            return Some(s);
                        }
                    }
                    None
                }
                serde_json::Value::Array(arr) => {
                    for v in arr {
                        if let Some(s) = find_prompt_in_value(v) {
                            return Some(s);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        find_prompt_in_value(json)
    }
}

pub fn apply_orientation(img: &DynamicImage, orientation: ExifOrientation) -> DynamicImage {
    match orientation {
        ExifOrientation::Normal => img.clone(),
        ExifOrientation::Rotate90 => img.rotate90(),
        ExifOrientation::Rotate180 => img.rotate180(),
        ExifOrientation::Rotate270 => img.rotate270(),
        ExifOrientation::FlipHorizontal => img.fliph(),
        ExifOrientation::FlipHorizontalRotate90 => {
            let rotated = img.rotate90();
            rotated.fliph()
        }
        ExifOrientation::FlipHorizontalRotate180 => {
            let rotated = img.rotate180();
            rotated.fliph()
        }
        ExifOrientation::FlipHorizontalRotate270 => {
            let rotated = img.rotate270();
            rotated.fliph()
        }
    }
}
