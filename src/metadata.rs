use image::DynamicImage;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use whatlang::detect;

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

fn is_english(text: &str) -> bool {
    let text = text.trim();
    if text.len() < 10 {
        return false;
    }
    if let Some(info) = detect(text) {
        return info.lang() == whatlang::Lang::Eng;
    }
    false
}

fn is_json_fragment(text: &str) -> bool {
    let text = text.trim();

    let brace_count = text.chars().filter(|&c| c == '{' || c == '}').count();
    let bracket_count = text.chars().filter(|&c| c == '[' || c == ']').count();
    let colon_count = text.chars().filter(|&c| c == ':').count();

    if brace_count > 3 || bracket_count > 3 {
        return true;
    }

    if colon_count > 2 && text.contains("\"") && text.contains(":") {
        if text.starts_with('{') || text.starts_with('[') {
            return true;
        }
        if text.contains("\"name\":") || text.contains("\"type\":") || text.contains("\"id\":") {
            return true;
        }
    }

    if text.contains("\"\"") || text.starts_with("\"") != text.ends_with("\"") {
        return true;
    }

    false
}

fn is_technical_string(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return true;
    }
    let text_lower = text.to_lowercase();
    if text.len() < 3 {
        return true;
    }
    if text
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        return true;
    }
    if is_json_fragment(text) {
        return true;
    }
    let technical_keywords = [
        "workflow",
        "last_node_id",
        "last_link_id",
        "class_type",
        "sampler_name",
        "scheduler",
        "noise_seed",
        "cfg",
        "steps",
        "width",
        "height",
        "vae_name",
        "unet_name",
        "model_name",
        "clip_name",
        "positive",
        "negative",
        "widgets",
        "values",
        "nodes",
        "links",
        "pos",
        "size",
        "flags",
        "mode",
        "order",
        "properties",
        "inputs",
        "outputs",
        "version",
        "config",
    ];
    for kw in &technical_keywords {
        if text_lower == *kw {
            return true;
        }
    }
    false
}

fn extract_strings_from_json(value: &JsonValue) -> Vec<String> {
    let mut strings = Vec::new();
    extract_strings_recursive(value, &mut strings);
    strings
}

fn extract_strings_recursive(value: &JsonValue, strings: &mut Vec<String>) {
    match value {
        JsonValue::Object(map) => {
            for (_key, val) in map {
                extract_strings_recursive(val, strings);
            }
        }
        JsonValue::Array(arr) => {
            for item in arr {
                extract_strings_recursive(item, strings);
            }
        }
        JsonValue::String(s) => {
            if s.len() > 5 && !s.chars().all(|c| c.is_whitespace()) {
                strings.push(s.clone());
            }
        }
        _ => {}
    }
}

fn find_prompts_in_strings(strings: Vec<String>) -> Vec<String> {
    let mut prompts: Vec<(String, i32)> = Vec::new();
    let mut seen = HashSet::new();

    for s in strings {
        let trimmed = s.trim();
        if trimmed.is_empty() || seen.contains(trimmed) {
            continue;
        }
        seen.insert(trimmed.to_string());

        if is_technical_string(trimmed) {
            continue;
        }

        let word_count = trimmed.split_whitespace().count();
        if word_count < 3 {
            continue;
        }

        let mut score = 0;

        if is_english(trimmed) {
            score += 100;
        }

        score += word_count as i32;

        if trimmed.len() > 50 {
            score += 20;
        }
        if trimmed.len() > 100 {
            score += 20;
        }

        let has_punctuation =
            trimmed.contains('.') || trimmed.contains(',') || trimmed.contains('!');
        if has_punctuation {
            score += 10;
        }

        let natural_words = [
            "the", "a", "an", "in", "on", "at", "with", "and", "or", "but", "is", "are", "was",
            "were", "be", "to", "of", "for", "from", "by",
        ];
        let text_lower = trimmed.to_lowercase();
        for word in &natural_words {
            if text_lower.contains(word) {
                score += 2;
            }
        }

        prompts.push((trimmed.to_string(), score));
    }

    prompts.sort_by(|a, b| b.1.cmp(&a.1));

    prompts.into_iter().map(|(s, _)| s).collect()
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

        let (exif, prompt) = match extension.as_str() {
            "jpg" | "jpeg" | "png" => (Self::read_exif_data(path), Self::read_png_prompt(path)),
            "webp" => (None, Self::read_webp_prompt(path)),
            _ => (None, None),
        };

        let orientation = exif
            .as_ref()
            .map(|e| e.orientation)
            .unwrap_or(ExifOrientation::Normal);

        ImageMetadata {
            orientation,
            prompt,
            exif,
        }
    }

    pub fn get_metadata_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        if let Some(ref exif) = self.exif {
            for (key, value) in exif.to_key_values() {
                lines.push(format!("{}: {}", key, value));
            }
        }

        if let Some(ref prompt) = self.prompt {
            lines.push("".to_string());
            lines.push("Prompt:".to_string());
            let max_chars = 80;
            let chars: Vec<char> = prompt.chars().collect();
            for chunk in chars.chunks(max_chars) {
                let chunk_str: String = chunk.iter().collect();
                lines.push(chunk_str);
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

        // First try reading EXIF data from PNG
        let mut cursor = std::io::Cursor::new(&bytes);
        if let Ok(exif) = exif::Reader::new().read_from_container(&mut cursor) {
            for field in exif.fields() {
                let value_str = field.display_value().to_string();

                let is_text_field = !value_str.contains("0x")
                    && value_str.len() > 3
                    && value_str.chars().any(|c| c.is_ascii_alphabetic());

                if is_text_field {
                    let mut all_strings: Vec<String> = Vec::new();

                    if let Ok(json_value) = serde_json::from_str::<JsonValue>(&value_str) {
                        let strings = extract_strings_from_json(&json_value);
                        all_strings.extend(strings);
                    } else {
                        all_strings.push(value_str.clone());
                    }

                    let prompts = find_prompts_in_strings(all_strings);
                    if let Some(prompt) = prompts.into_iter().next() {
                        return Some(prompt);
                    }
                }
            }
        }

        // Then try PNG text chunks
        let owned_bytes = bytes.to_vec();
        let png = match img_parts::png::Png::from_bytes(owned_bytes.into()) {
            Ok(p) => p,
            Err(_) => return None,
        };

        for chunk in png.chunks() {
            let kind = chunk.kind();
            if kind == *b"tEXt" || kind == *b"iTXt" || kind == *b"zTXt" {
                if let Ok(text) = std::str::from_utf8(chunk.contents()) {
                    if let Some(pos) = text.find('\0') {
                        let value = &text[pos + 1..];

                        let mut all_strings: Vec<String> = Vec::new();

                        if let Ok(json_value) = serde_json::from_str::<JsonValue>(value) {
                            let strings = extract_strings_from_json(&json_value);
                            all_strings.extend(strings);
                        } else {
                            all_strings.push(value.to_string());
                        }

                        let prompts = find_prompts_in_strings(all_strings);
                        if let Some(prompt) = prompts.into_iter().next() {
                            return Some(prompt);
                        }
                    }
                }
            }
        }

        None
    }

    fn read_webp_prompt(path: &Path) -> Option<String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut reader = BufReader::new(file);
        let mut bytes = Vec::new();
        if reader.read_to_end(&mut bytes).is_err() {
            return None;
        }

        let mut offset = 0;
        while offset < bytes.len().saturating_sub(12) {
            if bytes[offset..offset + 4] == [0x52, 0x49, 0x46, 0x46] {
                let chunk_size = u32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]) as usize;
                let chunk_type = std::str::from_utf8(&bytes[offset + 8..offset + 12]).unwrap_or("");

                if chunk_type == "WEBP" {
                    let mut inner = offset + 12;
                    while inner < offset + 12 + chunk_size && inner < bytes.len().saturating_sub(8)
                    {
                        let sub_type = std::str::from_utf8(&bytes[inner..inner + 4]).unwrap_or("");
                        let sub_size = u32::from_le_bytes([
                            bytes[inner + 4],
                            bytes[inner + 5],
                            bytes[inner + 6],
                            bytes[inner + 7],
                        ]) as usize;

                        if sub_type == "EXIF" {
                            let exif_data = &bytes[inner + 8..inner + 8 + sub_size];

                            let cursor = std::io::Cursor::new(exif_data);
                            let mut exif_reader = BufReader::new(cursor);

                            if let Ok(exif) =
                                exif::Reader::new().read_from_container(&mut exif_reader)
                            {
                                if let Some(field) =
                                    exif.get_field(exif::Tag::ImageDescription, exif::In::PRIMARY)
                                {
                                    let value = field.display_value().to_string();

                                    let unescaped = value
                                        .trim_matches('"')
                                        .replace("\\\"", "\"")
                                        .replace("\\n", "\n")
                                        .replace("\\t", "\t");

                                    let json_str = unescaped.trim_start_matches("Workflow:").trim();

                                    if json_str.starts_with('{') {
                                        if let Ok(json_value) =
                                            serde_json::from_str::<JsonValue>(json_str)
                                        {
                                            let strings = extract_strings_from_json(&json_value);
                                            let prompts = find_prompts_in_strings(strings);
                                            if let Some(prompt) = prompts.into_iter().next() {
                                                return Some(prompt);
                                            }
                                        }
                                    } else {
                                        if is_english(json_str) && json_str.len() > 20 {
                                            return Some(json_str.to_string());
                                        }
                                    }
                                }
                            }
                        }

                        inner += 8 + sub_size;
                        if sub_size % 2 != 0 {
                            inner += 1;
                        }
                    }
                }
                break;
            }
            offset += 1;
        }

        None
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
