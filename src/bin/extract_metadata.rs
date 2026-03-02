use img_parts::png::Png;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use whatlang::detect;

fn extract_png_text_chunks(buffer: &[u8]) -> Vec<(String, String)> {
    let mut chunks = Vec::new();

    let owned_buffer = buffer.to_vec();
    if let Ok(png) = Png::from_bytes(owned_buffer.into()) {
        for chunk in png.chunks() {
            let kind = chunk.kind();
            let kind_str = std::str::from_utf8(&kind).unwrap_or("");

            if kind_str == "tEXt" || kind_str == "iTXt" || kind_str == "zTXt" {
                let contents = chunk.contents();
                if let Ok(text) = std::str::from_utf8(contents) {
                    let parts: Vec<&str> = text.splitn(2, '\0').collect();
                    if parts.len() == 2 {
                        let keyword = parts[0].to_string();
                        let value = parts[1].to_string();
                        chunks.push((keyword, value));
                    }
                }
            }
        }
    }

    chunks
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

fn is_likely_prompt(text: &str) -> f32 {
    let text = text.trim();
    if text.len() < 8 {
        return 0.0;
    }

    let text_lower = text.to_lowercase();

    let skip_patterns = [
        "row 0 at top",
        "column 0 at left",
        "co-sited",
        "one-chip color area",
        "digital still camera",
        "directly photographed",
        "normal process",
        "auto exposure",
        "auto white balance",
        "standard",
        "unknown",
        "sRGB",
        "YCbCr",
        "inch",
        "pattern",
        "not fired",
        "digital camera",
        "finepix",
        "ver",
        "r98",
        "cm",
        "1/100",
        "400",
        "sos",
        "2.3",
        "ycbcr",
        "6.63",
        "2.28",
        "2.19",
        "normal program",
        "one-chip",
        "color area",
        "fujifilm",
        "finepix x100",
        "digital camera finepix",
        "exif",
        "tiff",
        "interoperability",
        "interop",
        "0x",
        "printim",
        "workflow",
        "revision",
        "last_node_id",
        "last_link_id",
        "nodes",
        "properties",
        "cnr_id",
        "Node name",
        "ue_properties",
        "widget_ue_connectable",
        "input_ue_unconnectable",
        "version",
        "enableTabs",
        "tabWidth",
        "tabXOffset",
        "hasSecondTab",
        "secondTabText",
        "secondTabOffset",
        "models",
        "directory",
        "input",
        "inputs",
        "outputs",
        "output",
        "name",
        "type",
        "links",
        "id",
        "pos",
        "size",
        "flags",
        "order",
        "mode",
        "class_type",
        "meta",
        "title",
        "extra",
        "config",
        "links_added_by_ue",
        "frontendVersion",
        "VHS_latentpreview",
        "VHS_KeepIntermediate",
        "workflowRendererVersion",
        "widget_idx_map",
        "ds",
        "scale",
        "offset",
        "groups",
        "bounding",
        "color",
        "font_size",
        "flux",
        "klein",
        "qwen",
        "safetensors",
        "huggingface",
        "resolve",
        "main",
        "split_files",
        "Comfy-Org",
        "ComfyUI",
        "rgthree",
        "klein9",
        "celluliteass",
        "chastityCage",
        "amateur_selfie",
        "sampler_name",
        "cfg",
        "steps",
        "noise_seed",
        "scheduler",
        "start_at_step",
        "end_at_step",
        "return_with_leftover_noise",
        "enable",
        "fixed",
        "beta",
        "disable",
        "unet_name",
        "weight_dtype",
        "default",
        "device",
        "vae_name",
        "filename_prefix",
        "filename_keys",
        "foldername_prefix",
        "foldername_keys",
        "delimiter",
        "save_job_data",
        "job_data_per_image",
        "job_custom_text",
        "save_metadata",
        "counter_digits",
        "counter_position",
        "one_counter_per_folder",
        "image_preview",
        "output_ext",
        "quality",
        "images",
        "latent_type",
        "width",
        "height",
        "auto_detect",
        "rescale_mode",
        "rescale_value",
        "batch_size",
        "rescaleValue",
        "scaling_slider_min",
        "scaling_slider_max",
        "scaling_slider_step",
        "megapixels_slider_min",
        "megapixels_slider_max",
        "megapixels_slider_step",
        "snapValue",
        "upscaleValue",
        "targetResolution",
        "targetMegapixels",
        "canvas_min_x",
        "canvas_max_x",
        "canvas_step_x",
        "canvas_min_y",
        "canvas_max_y",
        "canvas_step_y",
        "canvas_decimals_x",
        "canvas_decimals_y",
        "canvas_snap",
        "canvas_dots",
        "canvas_frame",
        "action_slider_snap_min",
        "action_slider_snap_max",
        "action_slider_snap_step",
        "selectedCategory",
        "selectedPreset",
        "customPresetsJSON",
        "preset_selector_mode",
        "visual",
        "manual_slider_min_w",
        "manual_slider_max_w",
        "manual_slider_step_w",
        "manual_slider_min_h",
        "manual_slider_max_h",
        "manual_slider_step_h",
        "section_actions_collapsed",
        "section_scaling_collapsed",
        "section_autoDetect_collapsed",
        "section_presets_collapsed",
        "dropdown_resolution_expanded",
        "dropdown_category_expanded",
        "dropdown_preset_expanded",
        "useCustomCalc",
        "autoFitOnChange",
        "autoResizeOnChange",
        "autoDetect",
    ];
    for pattern in &skip_patterns {
        if text_lower.contains(pattern) {
            return 0.0;
        }
    }

    let numeric_ratio: f32 =
        text.chars().filter(|c| c.is_ascii_digit()).count() as f32 / text.len() as f32;
    if numeric_ratio > 0.5 {
        return 0.0;
    }

    if text_lower.contains("0x") && text.len() > 20 {
        return 0.0;
    }

    if text.contains('\\') || (text.contains('/') && text.contains('.')) {
        return 0.0;
    }

    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == '"')
        .filter(|w: &&str| w.len() > 2)
        .collect();
    if words.len() < 3 {
        return 0.0;
    }

    let prompt_indicators = [
        "of",
        "a",
        "the",
        "in",
        "with",
        "and",
        "for",
        "on",
        "at",
        "to",
        "an",
        "by",
        "from",
        "as",
        "is",
        "was",
        "are",
        "were",
        "be",
        "been",
        "being",
        "photograph",
        "photo",
        "image",
        "picture",
        "portrait",
        "landscape",
        "style",
        "art",
        "digital",
        "illustration",
        "painting",
        "drawing",
        "anime",
        "manga",
        "cartoon",
        "sketch",
        "concept",
        "render",
        "3d",
        "cinematic",
        "dramatic",
        "epic",
        "vivid",
        "detailed",
        "high quality",
        "4k",
        "8k",
        "ultra",
        "professional",
        "masterpiece",
        "best quality",
        "realistic",
        "photorealistic",
        "hyperrealistic",
        "natural",
        "lighting",
        "colors",
        "warm",
        "cool",
        "bright",
        "dark",
        "moody",
        "atmospheric",
        "character",
        "person",
        "woman",
        "man",
        "girl",
        "boy",
        "child",
        "face",
        "eyes",
        "hair",
        "body",
        "clothing",
        "outfit",
        "dress",
        "shirt",
        "background",
        "setting",
        "environment",
        "interior",
        "exterior",
        "urban",
        "nature",
        "subject",
        "scene",
        "generate",
        "artwork",
        "creative",
        "fantasy",
        "scifi",
        "science",
        "fiction",
        "cyberpunk",
        "steampunk",
        "magic",
        "mystical",
        "ethereal",
        "serene",
        "peaceful",
        "tense",
        "beautiful",
        "stunning",
        "gorgeous",
        "amazing",
        "incredible",
        "aesthetic",
        "composition",
        "frame",
        "shot",
        "angle",
        "perspective",
        "bokeh",
        "depth",
        "field",
        "focus",
        "sharp",
        "blur",
        "sunlight",
        "moonlight",
        "neon",
        "glow",
        "shine",
        "reflection",
        "futuristic",
        "retro",
        "vintage",
        "classic",
        "modern",
        "minimalist",
        "abstract",
        "surreal",
        "dreamlike",
        "otherworldly",
    ];

    let mut indicator_count = 0;
    for indicator in &prompt_indicators {
        if text_lower.contains(indicator) {
            indicator_count += 1;
        }
    }

    let length_score = if text.len() > 50 { 1.0 } else { 0.5 };
    (indicator_count as f32 * 0.3 + length_score).min(10.0)
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

    if text.starts_with("0x") && text.len() > 4 {
        return true;
    }

    if is_json_fragment(text) {
        return true;
    }

    if text.contains('\\')
        && (text.contains(".safetensors")
            || text.contains(".ckpt")
            || text.contains(".pt")
            || text.contains(".pth"))
    {
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
        "pos",
        "size",
        "flags",
        "mode",
        "order",
        "nodes",
        "links",
        "groups",
        "properties",
        "inputs",
        "outputs",
        "version",
        "config",
        "directory",
        "filename",
        "foldername",
        "extension",
    ];

    for kw in &technical_keywords {
        if text_lower == *kw || text_lower.starts_with(kw) {
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

fn find_prompts_in_strings(strings: Vec<String>) -> Vec<(String, f32)> {
    let mut prompts: Vec<(String, f32)> = Vec::new();
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

        let score = is_likely_prompt(trimmed);

        if is_english(trimmed) || score > 2.0 {
            prompts.push((
                trimmed.to_string(),
                score + if is_english(trimmed) { 5.0 } else { 0.0 },
            ));
        }
    }

    prompts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    prompts
}

fn extract_metadata(path: &Path) -> Vec<String> {
    let mut metadata = Vec::new();
    let mut detected_prompts: Vec<String> = Vec::new();

    if let Ok(file) = File::open(path) {
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        if reader.read_to_end(&mut buffer).is_ok() && !buffer.is_empty() {
            metadata.push(format!("File: {}", path.display()));
            metadata.push(format!("File size: {} bytes", buffer.len()));
            metadata.push(String::new());

            let mut cursor = std::io::Cursor::new(&buffer);
            match exif::Reader::new().read_from_container(&mut cursor) {
                Ok(exif) => {
                    metadata.push("All EXIF fields:".to_string());
                    for field in exif.fields() {
                        let value_str = field.display_value().to_string();

                        let is_text_field = !value_str.contains("0x")
                            && value_str.len() > 3
                            && value_str.chars().any(|c| c.is_ascii_alphabetic());

                        if is_text_field {
                            metadata.push(format!("  {:?}: {}", field.tag, value_str));

                            let json_candidates: Vec<String> = if let Ok(json_value) =
                                serde_json::from_str::<JsonValue>(&value_str)
                            {
                                extract_strings_from_json(&json_value)
                            } else {
                                let mut candidates = Vec::new();
                                let mut in_string = false;
                                let mut current = String::new();
                                let mut escaped = false;

                                for c in value_str.chars() {
                                    if escaped {
                                        escaped = false;
                                        current.push(c);
                                        continue;
                                    }
                                    if c == '\\' {
                                        escaped = true;
                                        current.push(c);
                                        continue;
                                    }
                                    if c == '"' {
                                        in_string = !in_string;
                                        if !in_string && !current.is_empty() {
                                            candidates.push(current.clone());
                                            current.clear();
                                        }
                                        continue;
                                    }
                                    if in_string {
                                        current.push(c);
                                    }
                                }

                                let mut new_strings: Vec<String> = Vec::new();
                                for c in &candidates {
                                    if let Ok(json) =
                                        serde_json::from_str::<JsonValue>(&format!("\"{}\"", c))
                                    {
                                        if let JsonValue::String(s) = json {
                                            new_strings.push(s);
                                        }
                                    }
                                }
                                candidates.extend(new_strings);
                                candidates
                            };

                            let prompts = find_prompts_in_strings(json_candidates);
                            for (prompt, _score) in prompts {
                                if !detected_prompts.contains(&prompt) {
                                    detected_prompts.push(prompt);
                                }
                            }

                            if let Ok(json_value) = serde_json::from_str::<JsonValue>(&value_str) {
                                if let JsonValue::Object(_) = json_value {
                                    let strings = extract_strings_from_json(&json_value);
                                    let prompts = find_prompts_in_strings(strings);
                                    for (prompt, _score) in prompts {
                                        if !detected_prompts.contains(&prompt) {
                                            detected_prompts.push(prompt);
                                        }
                                    }
                                } else if let JsonValue::String(s) = json_value {
                                    if let Some(start) = s.find('{') {
                                        let rest = &s[start..];
                                        let mut brace_count = 0;
                                        let mut in_string = false;
                                        let mut escaped = false;
                                        let mut end = 0;

                                        for (i, c) in rest.chars().enumerate() {
                                            if escaped {
                                                escaped = false;
                                                continue;
                                            }
                                            if c == '\\' {
                                                escaped = true;
                                                continue;
                                            }
                                            if c == '"' {
                                                in_string = !in_string;
                                                continue;
                                            }
                                            if in_string {
                                                continue;
                                            }
                                            if c == '{' {
                                                brace_count += 1;
                                            } else if c == '}' {
                                                brace_count -= 1;
                                                if brace_count == 0 {
                                                    end = i;
                                                    break;
                                                }
                                            }
                                        }
                                        if end > 0 {
                                            if let Ok(inner_json) =
                                                serde_json::from_str::<JsonValue>(&rest[..end + 1])
                                            {
                                                let strings =
                                                    extract_strings_from_json(&inner_json);
                                                let prompts = find_prompts_in_strings(strings);
                                                for (prompt, _score) in prompts {
                                                    if !detected_prompts.contains(&prompt) {
                                                        detected_prompts.push(prompt);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            metadata.push(format!("  {:?}: <binary/data>", field.tag));
                        }
                    }
                }
                Err(e) => {
                    metadata.push(format!("EXIF parsing error: {:?}", e));
                }
            }

            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.ends_with(".png") {
                metadata.push(String::new());
                metadata.push("PNG text chunks:".to_string());

                let png_chunks = extract_png_text_chunks(&buffer);
                for (keyword, text) in &png_chunks {
                    metadata.push(format!("  {}: {}", keyword, text));

                    let mut all_strings: Vec<String> = Vec::new();

                    if let Ok(json_value) = serde_json::from_str::<JsonValue>(text) {
                        let strings = extract_strings_from_json(&json_value);
                        all_strings.extend(strings);
                    } else {
                        all_strings.push(text.clone());
                    }

                    let prompts = find_prompts_in_strings(all_strings);
                    for (prompt, _score) in prompts {
                        if !detected_prompts.contains(&prompt) {
                            detected_prompts.push(prompt);
                        }
                    }
                }

                if png_chunks.is_empty() {
                    metadata.push("  (no text chunks found)".to_string());
                }
            }
        } else {
            metadata.push("Error: Could not read file".to_string());
        }
    } else {
        metadata.push(format!("Error: Could not open {}", path.display()));
    }

    if !detected_prompts.is_empty() {
        metadata.push(String::new());
        metadata.push("=".repeat(50));
        metadata.push(format!(
            "Detected {} potential prompt(s):",
            detected_prompts.len()
        ));
        metadata.push("=".repeat(50));
        for (i, prompt) in detected_prompts.iter().enumerate() {
            metadata.push(format!("\nPrompt #{}:", i + 1));
            metadata.push(prompt.clone());
        }
        metadata.push("=".repeat(50));
    } else {
        metadata.push(String::new());
        metadata.push("No AI generation prompts detected in EXIF metadata.".to_string());
    }

    metadata
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <image_path> [image_path2 ...]", args[0]);
        eprintln!("Detects AI generation prompts from EXIF metadata.");
        std::process::exit(1);
    }
    for arg in &args[1..] {
        let path = Path::new(arg);
        let results = extract_metadata(path);
        for line in results {
            println!("{}", line);
        }
        println!();
    }
}
