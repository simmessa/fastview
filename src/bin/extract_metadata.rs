use serde_json::Value as JsonValue;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

fn is_likely_prompt(text: &str) -> f32 {
    let text = text.trim();
    if text.len() < 8 {
        return 0.0;
    }

    let text_lower = text.to_lowercase();

    // Skip common EXIF technical strings
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
        "mode",
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
        "action_slider_snap_min",
        "action_slider_snap_max",
        "action_slider_snap_step",
    ];
    for pattern in &skip_patterns {
        if text_lower.contains(pattern) {
            return 0.0;
        }
    }

    // Skip if mostly numbers
    let numeric_ratio: f32 =
        text.chars().filter(|c| c.is_ascii_digit()).count() as f32 / text.len() as f32;
    if numeric_ratio > 0.5 {
        return 0.0;
    }

    // Skip if contains hex prefixes
    if text_lower.contains("0x") && text.len() > 20 {
        return 0.0;
    }

    // Skip if looks like a file path
    if text.contains('\\') || (text.contains('/') && text.contains('.')) {
        return 0.0;
    }

    // Count words
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == '"')
        .filter(|w: &&str| w.len() > 2)
        .collect();
    if words.len() < 3 {
        return 0.0;
    }

    // Common prompt indicators
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
        "color",
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
        "fantasy",
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
        "cyberpunk",
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
        "dragon",
        "unicorn",
        "phoenix",
        "knight",
        "princess",
        "warrior",
        "forest",
        "mountain",
        "ocean",
        "river",
        "lake",
        "desert",
        "city",
        "street",
        "building",
        "house",
        "castle",
        "temple",
        "church",
        "sword",
        "shield",
        "armor",
        "magic",
        "spell",
        "potion",
        "robot",
        "alien",
        "spaceship",
        "planet",
        "galaxy",
        "star",
        "animal",
        "cat",
        "dog",
        "bird",
        "horse",
        "wolf",
        "lion",
        "tattoo",
        "makeup",
        "jewelry",
        "accessory",
        "fashion",
        "style",
        "uniform",
        "military",
        "shoes",
        "heels",
        "socks",
        "corridor",
        "salute",
        "standing",
        "caucasian",
        "redhead",
        "blonde",
        "american",
        "african",
        "arabian",
        "curly",
        "skirt",
        "skirts",
        "trouser",
        "jeans",
        "jacket",
        "coat",
        "suit",
        "gown",
        "vest",
        "blouse",
        "pant",
        "shorts",
        "sweater",
        "tshirt",
        "hoodie",
        "cap",
        "hat",
        "scarf",
        "glove",
        "belt",
        "bag",
        "purse",
        "wallet",
        "watch",
        "glasses",
        "sunglasses",
        "necklace",
        "earring",
        "bracelet",
        "ring",
        "anklet",
        "chain",
        "pin",
        "button",
        "buttonhole",
        "zipper",
        "pocket",
        "collar",
        "cuff",
        "hem",
        "seam",
        "stitch",
        "thread",
        "fabric",
        "material",
        "texture",
        "pattern",
        "print",
        "stripe",
        "check",
        "plaid",
        "floral",
        "polka",
        "dot",
        "geometric",
        "abstract",
        "solid",
        "gradient",
        "ombre",
        "tie-dye",
        "embroidered",
        "printed",
        "woven",
        "knitted",
        "crocheted",
        "sewn",
        "stitched",
        "quilted",
        "padded",
        "lined",
        "fleece",
        "velvet",
        "silk",
        "cotton",
        "linen",
        "wool",
        "leather",
        "suede",
        "denim",
        "canvas",
        "nylon",
        "polyester",
        "rayon",
        "spandex",
        "acrylic",
        "merino",
        "alpaca",
        "cashmere",
        "mohair",
        "angora",
        "bamboo",
        "modal",
        "tencel",
        "lyocell",
    ];

    let mut indicator_count = 0;
    for indicator in &prompt_indicators {
        if text_lower.contains(indicator) {
            indicator_count += 1;
        }
    }

    // Score based on indicator count and length
    let length_score = if text.len() > 50 { 1.0 } else { 0.5 };
    (indicator_count as f32 * 0.3 + length_score).min(10.0)
}

fn extract_prompts_from_json(root: &JsonValue) -> Vec<(String, f32)> {
    let mut prompts = Vec::new();
    extract_prompts_recursive(root, &mut prompts);
    prompts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    prompts
}

fn extract_prompts_recursive(value: &JsonValue, prompts: &mut Vec<(String, f32)>) {
    match value {
        JsonValue::Object(map) => {
            for (key, val) in map {
                if key == "text" {
                    if let JsonValue::String(text) = val {
                        prompts.push((text.clone(), 10.0)); // Always add text fields
                    }
                } else {
                    extract_prompts_recursive(val, prompts);
                }
            }
        }
        JsonValue::Array(arr) => {
            for item in arr {
                extract_prompts_recursive(item, prompts);
            }
        }
        _ => {}
    }
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

                            // Try to parse as JSON and extract prompts
                            if let Ok(json_value) = serde_json::from_str::<JsonValue>(&value_str) {
                                if let JsonValue::Object(_) = json_value {
                                    let prompts = extract_prompts_from_json(&json_value);
                                    for (prompt, _score) in prompts {
                                        if !detected_prompts.contains(&prompt) {
                                            detected_prompts.push(prompt);
                                        }
                                    }
                                } else if let JsonValue::String(s) = json_value {
                                    // Try to find inner JSON by looking for the first {
                                    if let Some(start) = s.find('{') {
                                        // Find matching } by counting braces
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
                                                let prompts =
                                                    extract_prompts_from_json(&inner_json);
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
