use std::collections::HashSet;

use derive_more::From;
use glam::Vec2;
use winit::{ dpi::PhysicalPosition, window::Window };
pub use winit::{ event::MouseButton, keyboard::KeyCode };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
pub enum InputButton {
    Key(KeyCode),
    Mouse(MouseButton),
    MouseEntered,
    MouseWheelUp,
    MouseWheelDown,
    WindowFocused,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
pub struct CursorGrabMode {
    pub hidden: bool,
    pub captured: bool,
}

impl CursorGrabMode {
    pub const VISIBLE: Self = Self { hidden: false, captured: false };
    pub const HIDDEN: Self = Self { hidden: true, captured: false };
    pub const CAPTURED: Self = Self { hidden: true, captured: true };
}

#[derive(Debug, Default)]
pub struct EngineInputState {
    pressed_buttons: HashSet<InputButton>,
    just_pressed_buttons: HashSet<InputButton>,
    just_released_buttons: HashSet<InputButton>,
    mouse_motion: Vec2,
    mouse_pos: Vec2,
    changed_mouse_pos: Option<Vec2>,
    cursor_grab_mode: CursorGrabMode,
}

impl EngineInputState {
    /// equivalent to quickly calling pressed and released
    pub(crate) fn register_tapped(&mut self, button: impl Into<InputButton>) {
        let button = button.into();
        self.just_pressed_buttons.insert(button);
        self.just_released_buttons.insert(button);
    }

    pub(crate) fn register_pressed(&mut self, button: impl Into<InputButton>) {
        let button = button.into();
        self.pressed_buttons.insert(button);
        self.just_pressed_buttons.insert(button);
    }

    pub(crate) fn register_released(&mut self, button: impl Into<InputButton>) {
        let button = button.into();
        self.pressed_buttons.remove(&button);
        self.just_pressed_buttons.remove(&button);
        self.just_released_buttons.remove(&button);
    }

    pub(crate) fn register_button_state(&mut self, button: impl Into<InputButton>, pressed: bool) {
        if pressed {
            self.register_pressed(button);
        }
        else {
            self.register_released(button);
        }
    }

    pub(crate) fn register_mouse_pos(&mut self, pos: Vec2) {
        self.mouse_pos = pos;
    }

    pub(crate) fn register_mouse_motion(&mut self, motion: Vec2) {
        self.mouse_motion += motion;
    }
}

impl EngineInputState {
    fn clear_deltas(&mut self) {
        self.just_pressed_buttons.clear();
        self.just_released_buttons.clear();
        self.mouse_motion = Vec2::ZERO;
    }

    pub(crate) fn pre_frame_apply_changes(&mut self, window: &Window) {
        if window.is_visible().unwrap_or(false) {
            // TODO: Fallback to Locked for unsuppored platforms ?
            //       seems annoying for the application
            window.set_cursor_grab(if self.cursor_grab_mode.captured {
                winit::window::CursorGrabMode::Confined
            } else {
                winit::window::CursorGrabMode::None
            }).expect("Supported");
            window.set_cursor_visible(!self.cursor_grab_mode.hidden);
        }

        if let Some(new_mouse_pos) = self.changed_mouse_pos.take() {
            let new_mouse_pos = new_mouse_pos.as_uvec2();
            window.set_cursor_position(PhysicalPosition::new(
                new_mouse_pos.x,
                new_mouse_pos.y,
            )).expect("Success");
            self.mouse_pos = new_mouse_pos.as_vec2();
        }

        self.clear_deltas();
    }
}

impl EngineInputState {
    pub fn pressed(&self, button: impl Into<InputButton>) -> bool {
        let button = button.into();
        self.pressed_buttons.contains(&button)
    }

    /// Wether this input was pressed in between the last frame
    pub fn just_pressed(&self, button: impl Into<InputButton>) -> bool {
        let button = button.into();
        self.just_pressed_buttons.contains(&button)
    }

    /// Wether this input was pressed in between the last frame
    pub fn just_released(&self, button: impl Into<InputButton>) -> bool {
        let button = button.into();
        self.just_released_buttons.contains(&button)
    }

    /// In screen space
    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    pub fn set_mouse_pos(&mut self, new_pos: Vec2) {
        self.changed_mouse_pos = Some(new_pos);
    }

    pub fn cursor_grab_mode(&self) -> CursorGrabMode {
        self.cursor_grab_mode
    }

    pub fn set_cursor_grab_mode(&mut self, mode: CursorGrabMode) {
        self.cursor_grab_mode = mode;
    }

    /// Accumulated mous motion since last frame
    /// (not the same as the difference of mouse pos)
    pub fn mouse_motion(&self) -> Vec2 {
        self.mouse_motion
    }
}
