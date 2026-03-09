use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RemoteInput {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    PlayPause,
    VolumeUp,
    VolumeDown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
}

pub trait VibeApp {
    /// Returns information about the app for the Shell's menu
    fn metadata(&self) -> AppMetadata;

    /// Called when the app is first loaded into memory
    fn on_init(&mut self);

    /// The Shell passes remote control signals to the active app here
    fn handle_input(&mut self, input: RemoteInput);

    /// Returns the UI state or HTML/RSX string for the Shell to render
    /// In 2026, we often return a JSON "State Tree" that the Shell's CSS/JS handles
    fn render(&self) -> String;

    /// Called when the user exits the app back to the main menu
    fn on_shutdown(&mut self);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VibeTheme {
    pub primary_color: String,
    pub background_vibe: String, // e.g., "glass", "solid", "gradient"
    pub accent_color: String,
    pub border_radius: String,
    pub font_family: String,
}
