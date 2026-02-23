use winit::{
    event::{WindowEvent, MouseScrollDelta},
    keyboard::{Key, NamedKey},
};

pub enum InputAction {
    None,
    NextImage,
    PrevImage,
    Zoom(f32),
    Pan(f32, f32),
    Click(f64, f64),
    Back,
    ActualSize,
    SelectUp,
    SelectDown,
    SelectLeft,
    SelectRight,
    OpenSelected,
    PageUp,
    PageDown,
}

pub struct InputHandler {
    pub mouse_down: bool,
    pub last_mouse_x: f64,
    pub last_mouse_y: f64,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            mouse_down: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> InputAction {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    return self.handle_keyboard_input(event);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                };
                return InputAction::Zoom(amount);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let dx = (position.x - self.last_mouse_x) as f32;
                let dy = (position.y - self.last_mouse_y) as f32;
                self.last_mouse_x = position.x;
                self.last_mouse_y = position.y;
                if self.mouse_down {
                    return InputAction::Pan(dx, dy);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Left {
                    self.mouse_down = *state == winit::event::ElementState::Pressed;
                    if !self.mouse_down {
                        // Click on release
                        return InputAction::Click(self.last_mouse_x, self.last_mouse_y);
                    }
                }
            }
            _ => {}
        }
        InputAction::None
    }

    fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) -> InputAction {
        match &event.logical_key {
            Key::Named(key) => {
                match key {
                    NamedKey::ArrowRight => return InputAction::SelectRight,
                    NamedKey::ArrowLeft => return InputAction::SelectLeft,
                    NamedKey::ArrowUp => return InputAction::SelectUp,
                    NamedKey::ArrowDown => return InputAction::SelectDown,
                    NamedKey::MediaTrackNext => return InputAction::NextImage,
                    NamedKey::MediaTrackPrevious => return InputAction::PrevImage,
                    NamedKey::Backspace | NamedKey::Escape => return InputAction::Back,
                    NamedKey::Enter => return InputAction::OpenSelected,
                    NamedKey::PageUp => return InputAction::PageUp,
                    NamedKey::PageDown => return InputAction::PageDown,
                    _ => {}
                }
            }
            Key::Character(c) => {
                if c == "1" {
                    return InputAction::ActualSize;
                }
            }
            _ => {}
        }
        InputAction::None
    }
}