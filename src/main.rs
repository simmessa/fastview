#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod renderer;
mod image_loader;
mod input_handler;
mod cache_manager;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId, UserAttentionType},
};
use std::path::{PathBuf, Path};
use std::sync::{Arc};
use std::thread;
use std::io::{Read, Write};
use crossbeam_channel::{unbounded, Sender, Receiver};
use image::{RgbaImage, Rgba};
use imageproc::drawing::{draw_text_mut, draw_filled_rect_mut};
use imageproc::rect::Rect;
use ab_glyph::{FontArc, PxScale};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};

use renderer::Renderer;
use image_loader::{ImageLoader, FileItem};
use input_handler::{InputHandler, InputAction};
use cache_manager::{CacheManager, WindowSettings};

#[derive(PartialEq)]
enum ViewMode {
    Grid,
    Single,
}

#[derive(Debug)]
enum UserEvent {
    OpenPath(PathBuf),
}

struct LoaderRequest {
    path: PathBuf,
    index: usize,
    is_directory: bool,
}

struct LoaderResponse {
    index: usize,
    image: RgbaImage,
}

struct AppState {
    window: Arc<Window>,
    renderer: Renderer,
    image_loader: ImageLoader,
    input_handler: InputHandler,
    cache: CacheManager,
    mode: ViewMode,
    
    // Background loading
    loader_tx: Sender<Vec<LoaderRequest>>,
    response_rx: Receiver<LoaderResponse>,
    visible_indices_tx: Sender<Vec<usize>>,

    // Zoom state
    saved_zoom: f32,
    is_actual_size: bool,

    // Grid selection
    selected_index: usize,
}

impl AppState {
    fn new(window: Window, event_loop_proxy: EventLoopProxy<UserEvent>, cache: CacheManager) -> AppState {
        let window = Arc::new(window);
        let size = window.inner_size();
        
        let args: Vec<String> = std::env::args().collect();
        let input_path = if args.len() > 1 {
            PathBuf::from(&args[1])
        } else {
            PathBuf::from(".")
        };

        // Start File System scan in parallel with WGPU setup
        let (init_tx, init_rx) = unbounded::<ImageLoader>();
        let input_path_thread = input_path.clone();
        thread::spawn(move || {
            let input_path = std::fs::canonicalize(&input_path_thread).unwrap_or(input_path_thread);
            let (loader_path, _) = if input_path.is_file() {
                (input_path.parent().unwrap_or(Path::new(".")).to_path_buf(), Some(input_path.clone()))
            } else {
                (input_path, None)
            };
            let loader = ImageLoader::new(loader_path);
            let _ = init_tx.send(loader);
        });

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).expect("Failed to create surface");

        let adapter = futures_lite::future::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        )).expect("Failed to find an appropriate adapter");

        let (device, queue) = futures_lite::future::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )).expect("Failed to create device");

        let renderer = Renderer::new(device, queue, adapter, surface, size.width, size.height);

        let input_handler = InputHandler::new();

        // Setup background loader channels
        let (loader_tx, loader_rx) = unbounded::<Vec<LoaderRequest>>();
        let (response_tx, response_rx) = unbounded::<LoaderResponse>();
        let (visible_indices_tx, visible_indices_rx) = unbounded::<Vec<usize>>();
        
        // Wait for FS init
        let image_loader = init_rx.recv().expect("Failed to initialize FS");
        let initial_file = if args.len() > 1 {
            let p = PathBuf::from(&args[1]);
            if p.is_file() { Some(std::fs::canonicalize(&p).unwrap_or(p)) } else { None }
        } else {
            None
        };

        // Spawn background thread for image loading
        let cache_for_thread = cache.clone_db_handle(); 
        thread::spawn(move || {
            let mut pending_requests: Vec<LoaderRequest> = Vec::new();
            let mut visible_indices: Vec<usize> = Vec::new();
            let mut font: Option<FontArc> = None;

            loop {
                // Check for new requests
                while let Ok(mut requests) = loader_rx.try_recv() {
                    pending_requests.append(&mut requests);
                }

                // Check for visible update
                while let Ok(visible) = visible_indices_rx.try_recv() {
                    visible_indices = visible;
                }

                if pending_requests.is_empty() {
                    thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                // Lazy load font on first use
                if font.is_none() {
                    font = std::fs::read("C:\\Windows\\Fonts\\arial.ttf")
                        .ok()
                        .and_then(|data| FontArc::try_from_vec(data).ok());
                }

                // Re-prioritize: items in visible_indices first
                pending_requests.sort_by(|a, b| {
                    let a_visible = visible_indices.contains(&a.index);
                    let b_visible = visible_indices.contains(&b.index);
                    match (a_visible, b_visible) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.index.cmp(&b.index),
                    }
                });

                let request = pending_requests.remove(0);
                let mut thumb_opt: Option<RgbaImage> = None;

                if request.is_directory {
                    let mut img = RgbaImage::new(256, 256);
                    for p in img.pixels_mut() {
                        *p = Rgba([30, 40, 60, 255]);
                    }
                    draw_filled_rect_mut(&mut img, Rect::at(40, 40).of_size(176, 176), Rgba([200, 160, 40, 255]));
                    thumb_opt = Some(img);
                } else {
                    if let Some(img) = cache_for_thread.get_thumbnail(&request.path) {
                        thumb_opt = Some(img);
                    } else if let Some(img) = ImageLoader::load_dynamic_image_path(&request.path) {
                        let thumb = img.resize_to_fill(256, 256, image::imageops::FilterType::Triangle).to_rgba8();
                        cache_for_thread.set_thumbnail(&request.path, &thumb);
                        thumb_opt = Some(thumb);
                    }
                }

                if let Some(mut thumb) = thumb_opt {
                    if request.is_directory {
                        if let Some(font) = &font {
                            let text = request.path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let scale = PxScale::from(18.0);
                            draw_filled_rect_mut(&mut thumb, Rect::at(0, 220).of_size(256, 36), Rgba([0, 0, 0, 180]));
                            draw_text_mut(&mut thumb, Rgba([255, 255, 255, 255]), 10, 228, scale, font, &text);
                        }
                    }
                    let _ = response_tx.send(LoaderResponse { index: request.index, image: thumb });
                }
            }
        });

        // Spawn IPC listener thread
        thread::spawn(move || {
            let name = "fastview_ipc";
            let name = if NameTypeSupport::query().paths_supported() {
                format!("/tmp/{}.sock", name)
            } else {
                name.to_string()
            };

            let listener = match LocalSocketListener::bind(name.clone()) {
                Ok(l) => l,
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                    // Try to re-bind if previous instance crashed
                    let _ = std::fs::remove_file(&name);
                    LocalSocketListener::bind(name).expect("Failed to bind IPC socket")
                }
                Err(e) => panic!("IPC bind error: {}", e),
            };

            for conn in listener.incoming().filter_map(|c| c.ok()) {
                let mut conn = conn;
                let mut buf = String::new();
                if conn.read_to_string(&mut buf).is_ok() {
                    let path = PathBuf::from(buf.trim());
                    let _ = event_loop_proxy.send_event(UserEvent::OpenPath(path));
                }
            }
        });

        let mut app_state = AppState {
            window,
            renderer,
            image_loader,
            input_handler,
            cache,
            mode: ViewMode::Grid,
            loader_tx,
            response_rx,
            visible_indices_tx,
            saved_zoom: 1.0,
            is_actual_size: false,
            selected_index: 0,
        };
        
        // Sync renderer mode and load grid
        app_state.renderer.set_view_mode(true);
        app_state.load_grid();

        if let Some(file_path) = initial_file {
            app_state.open_image_internal(&file_path);
        }

        app_state.update_window_title();
        app_state.window.request_redraw();
        app_state
    }

    fn open_path(&mut self, path: PathBuf) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        
        if path.is_file() {
            let parent = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            self.image_loader.set_path(parent);
            self.load_grid();
            self.open_image_internal(&path);
        } else {
            self.image_loader.set_path(path);
            self.load_grid();
            self.mode = ViewMode::Grid;
            self.renderer.set_view_mode(true);
        }
        
        self.update_window_title();
        self.window.request_redraw();
        
        // Bring to foreground
        self.window.set_minimized(false);
        self.window.focus_window();
        self.window.request_user_attention(Some(UserAttentionType::Critical));
    }

    fn open_image_internal(&mut self, file_path: &Path) {
        if let Some(img) = self.image_loader.open_image(file_path) {
            self.selected_index = self.image_loader.get_items().iter().position(|item| {
                match item {
                    FileItem::Image(p) => p == file_path,
                    _ => false,
                }
            }).unwrap_or(0);
            
            self.renderer.update_texture(&img);
            self.set_zoom_to_fit();
            self.renderer.set_view_mode(false);
            self.mode = ViewMode::Single;
        }
    }

    fn set_zoom_to_fit(&mut self) {
        let img_size = self.renderer.get_image_size();
        let win_size = self.renderer.get_window_size();
        
        if img_size[0] <= 0.0 || img_size[1] <= 0.0 {
            self.renderer.set_zoom(1.0);
            return;
        }

        let ia = img_size[0] / img_size[1];
        let wa = win_size[0] / win_size[1];
        
        let zoom = if wa > ia {
            win_size[1] / img_size[1]
        } else {
            win_size[0] / img_size[0]
        };
        
        self.renderer.set_zoom(zoom.min(1.0));
    }

    fn load_grid(&mut self) {
        self.selected_index = 0;
        self.renderer.clear_grid();
        let items = self.image_loader.get_items().to_vec();
        let mut requests = Vec::new();
        
        for (i, item) in items.iter().enumerate() {
            match item {
                FileItem::Directory(p) => {
                    self.renderer.add_grid_item(p.clone(), true, None);
                    requests.push(LoaderRequest { path: p.clone(), index: i, is_directory: true });
                }
                FileItem::Image(p) => {
                    self.renderer.add_grid_item(p.clone(), false, None);
                    requests.push(LoaderRequest { path: p.clone(), index: i, is_directory: false });
                }
            }
        }
        
        let _ = self.loader_tx.send(requests);
        self.update_viewport();
    }

    fn update_viewport(&mut self) {
        if self.mode != ViewMode::Grid { return; }
        
        let grid_size = 250.0;
        let spacing = 20.0;
        let window_size = self.renderer.get_window_size();
        let cols = (window_size[0] / (grid_size + spacing)).floor().max(1.0) as u32;
        let scroll = self.renderer.grid_scroll;
        
        let start_row = ((-scroll - spacing) / (grid_size + spacing)).floor().max(0.0) as u32;
        let end_row = ((-scroll + window_size[1] + spacing) / (grid_size + spacing)).ceil() as u32;
        
        let start_idx = (start_row * cols) as usize;
        let end_idx = (end_row * cols) as usize;
        
        let visible: Vec<usize> = (start_idx..end_idx).collect();
        let _ = self.visible_indices_tx.send(visible);
    }

    fn handle_window_event(&mut self, event: WindowEvent) {
        while let Ok(msg) = self.response_rx.try_recv() {
            self.renderer.update_grid_item_texture(msg.index, &msg.image);
            self.window.request_redraw();
        }

        let input_action = self.input_handler.handle_window_event(&event);
        match input_action {
            InputAction::None => {}
            InputAction::NextImage => {
                if self.mode == ViewMode::Single {
                    if let Some(img) = self.image_loader.next_image() {
                        self.is_actual_size = false;
                        self.renderer.set_filtering(false, None);
                        self.renderer.update_texture(&img);
                        self.set_zoom_to_fit();
                        self.update_window_title();
                        self.window.request_redraw();
                    }
                } else if self.mode == ViewMode::Grid {
                    self.move_selection(1, 0);
                }
            }
            InputAction::PrevImage => {
                if self.mode == ViewMode::Single {
                    if let Some(img) = self.image_loader.prev_image() {
                        self.is_actual_size = false;
                        self.renderer.set_filtering(false, None);
                        self.renderer.update_texture(&img);
                        self.set_zoom_to_fit();
                        self.update_window_title();
                        self.window.request_redraw();
                    }
                } else if self.mode == ViewMode::Grid {
                    self.move_selection(-1, 0);
                }
            }
            InputAction::Zoom(amount) => {
                self.renderer.zoom(amount);
                if self.mode == ViewMode::Grid {
                    self.update_viewport();
                }
                self.window.request_redraw();
            }
            InputAction::Pan(dx, dy) => {
                self.renderer.pan(dx, dy);
                self.window.request_redraw();
            }
            InputAction::Click(x, y) => {
                if self.mode == ViewMode::Grid {
                    let grid_size = 250.0;
                    let spacing = 20.0;
                    let scroll = self.renderer.grid_scroll;
                    
                    let col = ((x - spacing as f64) / (grid_size + spacing) as f64).floor() as i32;
                    let row = (((y - scroll as f64) - spacing as f64) / (grid_size + spacing) as f64).floor() as i32;
                    
                    let window_width = self.renderer.get_window_size()[0];
                    let cols = (window_width / (grid_size + spacing)).floor().max(1.0) as u32;
                    
                    if col >= 0 && col < cols as i32 && row >= 0 {
                        let index = (row as u32 * cols + col as u32) as usize;
                        let item_opt = self.image_loader.get_items().get(index).cloned();
                        if let Some(item) = item_opt {
                            self.selected_index = index;
                            match item {
                                FileItem::Directory(p) => {
                                    self.image_loader.set_path(p);
                                    self.load_grid();
                                }
                                FileItem::Image(p) => {
                                    if let Some(img) = self.image_loader.open_image(&p) {
                                        self.renderer.update_texture(&img);
                                        self.set_zoom_to_fit();
                                        self.renderer.set_view_mode(false);
                                        self.mode = ViewMode::Single;
                                    }
                                }
                            }
                            self.update_window_title();
                            self.window.request_redraw();
                        }
                    }
                }
            }
            InputAction::Back => {
                if self.mode == ViewMode::Single {
                    self.is_actual_size = false;
                    self.renderer.set_filtering(false, None);
                    self.mode = ViewMode::Grid;
                    self.renderer.set_view_mode(true);
                } else {
                    let mut path = self.image_loader.get_path().to_path_buf();
                    if path.pop() {
                        self.image_loader.set_path(path);
                        self.load_grid();
                    }
                }
                self.update_window_title();
                self.window.request_redraw();
            }
            InputAction::ActualSize => {
                if self.mode == ViewMode::Single {
                    if !self.is_actual_size {
                        self.saved_zoom = self.renderer.get_zoom();
                        self.is_actual_size = true;
                        self.renderer.set_zoom(1.0); 
                    } else {
                        self.is_actual_size = false;
                        self.renderer.set_zoom(self.saved_zoom);
                    }
                    
                    if let Some(img) = self.image_loader.load_current_image() {
                         self.renderer.set_filtering(self.is_actual_size, Some(&img));
                    }
                    self.window.request_redraw();
                }
            }
            InputAction::SelectUp => {
                if self.mode == ViewMode::Grid {
                    self.move_selection(0, -1);
                }
            }
            InputAction::SelectDown => {
                if self.mode == ViewMode::Grid {
                    self.move_selection(0, 1);
                }
            }
            InputAction::SelectLeft => {
                if self.mode == ViewMode::Grid {
                    self.move_selection(-1, 0);
                } else if self.mode == ViewMode::Single {
                    if let Some(img) = self.image_loader.prev_image() {
                        self.renderer.update_texture(&img);
                        self.set_zoom_to_fit();
                        self.update_window_title();
                        self.window.request_redraw();
                    }
                }
            }
            InputAction::SelectRight => {
                if self.mode == ViewMode::Grid {
                    self.move_selection(1, 0);
                } else if self.mode == ViewMode::Single {
                    if let Some(img) = self.image_loader.next_image() {
                        self.renderer.update_texture(&img);
                        self.set_zoom_to_fit();
                        self.update_window_title();
                        self.window.request_redraw();
                    }
                }
            }
            InputAction::OpenSelected => {
                if self.mode == ViewMode::Grid {
                    let item_opt = self.image_loader.get_items().get(self.selected_index).cloned();
                    if let Some(item) = item_opt {
                        match item {
                            FileItem::Directory(p) => {
                                self.image_loader.set_path(p);
                                self.load_grid();
                                self.update_window_title();
                            }
                            FileItem::Image(p) => {
                                if let Some(img) = self.image_loader.open_image(&p) {
                                    self.renderer.update_texture(&img);
                                    self.set_zoom_to_fit();
                                    self.renderer.set_view_mode(false);
                                    self.mode = ViewMode::Single;
                                    self.update_window_title();
                                }
                            }
                        }
                        self.window.request_redraw();
                    }
                }
            }
            InputAction::PageUp => {
                if self.mode == ViewMode::Grid {
                    self.move_selection_by_page(-1);
                }
            }
            InputAction::PageDown => {
                if self.mode == ViewMode::Grid {
                    self.move_selection_by_page(1);
                }
            }
            InputAction::Exit => {
                std::process::exit(0);
            }
        }

        match &event {
            WindowEvent::CloseRequested => {
                std::process::exit(0);
            }
            WindowEvent::Resized(new_size) => {
                self.renderer.resize(new_size.width, new_size.height);
                self.save_window_state();
                self.update_viewport();
                self.window.request_redraw();
            }
            WindowEvent::Moved(_) => {
                self.save_window_state();
            }
            WindowEvent::RedrawRequested => {
                self.renderer.render(self.mode == ViewMode::Grid, if self.mode == ViewMode::Grid { Some(self.selected_index) } else { None });
            }
            _ => {}
        }
    }

    fn save_window_state(&self) {
        if let Ok(pos) = self.window.outer_position() {
            let size = self.window.inner_size();
            self.cache.set_window_settings(&WindowSettings {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            });
        }
    }

    fn move_selection(&mut self, dx: i32, dy: i32) {
        let total_items = self.image_loader.get_items().len();
        if total_items == 0 { return; }

        let grid_size = 250.0;
        let spacing = 20.0;
        let window_width = self.renderer.get_window_size()[0];
        let cols = (window_width / (grid_size + spacing)).floor().max(1.0) as u32;

        let mut index = self.selected_index as i32;
        if dx != 0 {
            index += dx;
        }
        if dy != 0 {
            index += dy * cols as i32;
        }

        if index >= 0 && index < total_items as i32 {
            self.selected_index = index as usize;
            self.renderer.scroll_to_item(self.selected_index);
            self.update_viewport(); // Ensure thumbnails start loading for new view
            self.window.request_redraw();
        }
    }

    fn move_selection_by_page(&mut self, dir: i32) {
        let total_items = self.image_loader.get_items().len();
        if total_items == 0 { return; }

        let grid_size = 250.0;
        let spacing = 20.0;
        let [win_width, win_height] = self.renderer.get_window_size();
        
        let cols = (win_width / (grid_size + spacing)).floor().max(1.0) as u32;
        let rows_per_page = (win_height / (grid_size + spacing)).floor().max(1.0) as u32;
        let items_per_page = (rows_per_page * cols) as i32;

        let mut index = self.selected_index as i32 + dir * items_per_page;
        index = index.clamp(0, total_items as i32 - 1);

        if self.selected_index != index as usize {
            self.selected_index = index as usize;
            self.renderer.scroll_to_item(self.selected_index);
            self.update_viewport();
            self.window.request_redraw();
        }
    }

    fn update_window_title(&self) {
        let mut title = String::from("FastView");
        if self.mode == ViewMode::Grid {
            title.push_str(" - Browsing: ");
            title.push_str(self.image_loader.get_path().to_string_lossy().as_ref());
        } else {
            title.push_str(&format!(
                " - [{}/{}]",
                self.image_loader.get_current_index() + 1,
                self.image_loader.get_image_count()
            ));
        }
        self.window.set_title(&title);
    }
}

struct App {
    state: Option<AppState>,
    event_loop_proxy: EventLoopProxy<UserEvent>,
    cache: CacheManager,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let mut window_attributes = Window::default_attributes()
                .with_title("FastView")
                .with_inner_size(LogicalSize::new(1280, 720));
            
            // Restore window state
            if let Some(settings) = self.cache.get_window_settings() {
                window_attributes = window_attributes
                    .with_inner_size(LogicalSize::new(settings.width, settings.height))
                    .with_position(winit::dpi::PhysicalPosition::new(settings.x, settings.y));
            }
            
            let window = event_loop.create_window(window_attributes).expect("Failed to create window");
            self.state = Some(AppState::new(window, self.event_loop_proxy.clone(), self.cache.clone()));
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            state.handle_window_event(event);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        if let Some(state) = &mut self.state {
            match event {
                UserEvent::OpenPath(path) => {
                    state.open_path(path);
                }
            }
        }
    }
}

fn main() {
    env_logger::init();
    
    let args: Vec<String> = std::env::args().collect();
    let name = "fastview_ipc";
    let name = if NameTypeSupport::query().paths_supported() {
        format!("/tmp/{}.sock", name)
    } else {
        name.to_string()
    };

    // Try to connect to existing instance
    if let Ok(mut stream) = LocalSocketStream::connect(name.clone()) {
        let path = if args.len() > 1 {
            args[1].clone()
        } else {
            ".".to_string()
        };
        let _ = stream.write_all(path.as_bytes());
        return;
    }

    let cache = CacheManager::new();
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    let event_loop_proxy = event_loop.create_proxy();
    let mut app = App { state: None, event_loop_proxy, cache };
    event_loop.run_app(&mut app).unwrap();
}