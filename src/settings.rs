use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub x: i32,
    pub y: i32,
    pub opacity: u8,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            x: 100,
            y: 100,
            opacity: 200,
        }
    }
}

pub fn load_settings() -> Settings {
    let mut file = match File::open("settings.json") {
        Ok(file) => file,
        Err(_) => return Settings::default(),
    };

    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        return Settings::default();
    }

    serde_json::from_str(&content).unwrap_or_else(|_| Settings::default())
}

pub fn save_settings(settings: &Settings) {
    match serde_json::to_string_pretty(settings) {
        Ok(content) => {
            if let Ok(mut file) = File::create("settings.json") {
                let _ = file.write_all(content.as_bytes());
            }
        }
        Err(e) => eprintln!("Failed to serialize settings: {:?}", e),
    }
}

pub fn update_opacity(opacity: u8) {
    let mut settings = load_settings();
    settings.opacity = opacity;
    save_settings(&settings);
}

pub fn update_location(x: i32, y: i32) {
    let mut settings = load_settings();
    settings.x = x;
    settings.y = y;
    save_settings(&settings);
}
