# FastView

A fast, lightweight image viewer written in Rust with GPU-accelerated rendering.

## About

FastView was created entirely by AI agents. The author simply asked the agents nicely to build an image viewer, and they coded the entire application from scratch - including all features, UI, rendering pipeline, and metadata extraction.

## Features

### Core Features
- **Fast GPU-accelerated image rendering** using wgpu/WebGPU
- **Grid view** for browsing image folders
- **Single image view** with smooth zoom and pan
- **Support for multiple image formats**: JPEG, PNG, WebP
- **EXIF metadata extraction** from images
- **AI prompt detection** from generated images (ComfyUI, FLUX, etc.)

### Image Viewing
- Smooth zoom with mouse wheel
- Pan by dragging with mouse
- Actual size (1:1) zoom mode
- Remembers window position and size between sessions

### AI Metadata Extraction
- Automatically detects prompts from AI-generated images
- Supports various AI tools: ComfyUI, FLUX, Stable Diffusion, etc.
- Extracts prompts stored in EXIF data and PNG text chunks
- Natural language filtering to separate prompts from technical JSON

### Keyboard Shortcuts

#### Navigation
| Key | Action |
|-----|--------|
| Arrow Keys | Move selection in grid view |
| Enter | Open selected image |
| Backspace | Go back to parent folder |
| Page Up / Page Down | Scroll grid |
| Mouse Wheel | Scroll grid |

#### Image Viewing
| Key | Action |
|-----|--------|
| Media Next (or Right Arrow in single view) | Next image |
| Media Previous (or Left Arrow in single view) | Previous image |
| 1 | Actual size (1:1 zoom) |
| Mouse Wheel | Zoom in/out in single view |
| C | Copy extracted AI prompt to clipboard |

#### Display
| Key | Action |
|-----|--------|
| M | Toggle metadata overlay |
| H | Toggle help overlay |
| Escape | Close application |

## Technical Details

### Technology Stack
- **Language**: Rust
- **Rendering**: wgpu (WebGPU)
- **UI Framework**: winit
- **Image Processing**: image crate
- **Metadata**: kamadak-exif, img-parts
- **Language Detection**: whatlang (for prompt extraction)
- **Clipboard**: arboard

### Building

You shouldn't build this, since binaries are provided for Windows [here](/releases).

```bash
# Build the main application
cargo build --bin fastview

# Build the metadata extraction tool
cargo build --bin extract_metadata

# Run in release mode for best performance
cargo build --release --bin fastview
```

### Metadata Extraction Tool

FastView includes a standalone tool for extracting AI prompts from images:

```bash
# Extract metadata from an image
cargo run --bin extract_metadata -- /path/to/image.png
```

This tool reads EXIF data and PNG text chunks to detect and extract AI generation prompts.

## Caching System

FastView uses an aggressive caching system to ensure maximum speed. Here's how it works:

### Thumbnail Cache
- Every image you view gets its thumbnail cached to disk
- Thumbnails are stored as uncompressed RGBA data for instant loading
- The cache is stored in `%LOCALAPPDATA%\fastview\`

### WARNING: Disk Space Usage

**This cache will use A LOT of disk space!** 

Since FastView prioritizes speed above all else, it caches full-resolution thumbnails for every image you view. A typical folder with a few thousand images can easily use several gigabytes of cache. This is intentional - the goal is instant image loading, not disk efficiency.

If you need to free up space, you can safely delete the cache folder:
```
%LOCALAPPDATA%\fastview\
```

But hey, it will be recreated... There's no cap on the size of this folder atm.

### Window Settings
FastView also caches your window position and size so it restores exactly where you left off each time you launch.

## License

See .license_report.md for dependency license information.
