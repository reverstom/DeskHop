#![windows_subsystem = "windows"]

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use log::{debug, error, info};
use simplelog::*;
use std::fs::{create_dir_all, File};
use std::path::Path;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use winit::event_loop::{ControlFlow, EventLoop};

mod desktop_manager;
mod osd_window;
mod settings;

fn init_logging() {
    let log_dir = Path::new("logs");
    if let Err(e) = create_dir_all(log_dir) {
        eprintln!("Failed to create logs directory: {:?}", e);
        return;
    }

    let log_path = log_dir.join("deskhop.log");

    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Debug,
        Config::default(),
        File::create(log_path).unwrap(),
    )])
    .unwrap();
    info!("Logging initialized in logs/deskhop.log");
}

fn main() {
    init_logging();
    info!("DeskHop starting...");

    let event_loop = match EventLoop::new() {
        Ok(el) => {
            info!("Event loop created");
            el
        }
        Err(e) => {
            error!("Failed to create event loop: {:?}", e);
            return;
        }
    };

    // Load settings
    info!("Loading settings...");
    let current_settings = settings::load_settings();
    info!("Settings loaded: {:?}", current_settings);

    // Initialize Desktop Manager
    info!("Initializing Desktop Manager...");
    let desktop_manager = desktop_manager::DesktopManager::new();
    info!("Desktop Manager initialized");

    // Initialize OSD Window
    info!("Initializing OSD Window...");
    let osd_window = match osd_window::OsdWindow::new(
        current_settings.x,
        current_settings.y,
        current_settings.opacity,
    ) {
        Ok(osd) => {
            info!("OSD Window created");
            osd
        }
        Err(e) => {
            error!("Failed to create OSD window: {:?}", e);
            return;
        }
    };
    osd_window.show();
    info!("OSD Window shown");

    // Initialize Hotkey Manager
    info!("Initializing Hotkey Manager...");
    let hotkey_manager = match GlobalHotKeyManager::new() {
        Ok(hm) => hm,
        Err(e) => {
            error!("Failed to initialize hotkey manager: {:?}", e);
            return;
        }
    };
    let mut hotkeys = Vec::new();

    // Register Alt + 1 to Alt + 9
    info!("Registering hotkeys...");
    for i in 1..=9 {
        let code = match i {
            1 => Code::Digit1,
            2 => Code::Digit2,
            3 => Code::Digit3,
            4 => Code::Digit4,
            5 => Code::Digit5,
            6 => Code::Digit6,
            7 => Code::Digit7,
            8 => Code::Digit8,
            9 => Code::Digit9,
            _ => unreachable!(),
        };
        let hotkey = HotKey::new(Some(Modifiers::ALT), code);
        if let Err(e) = hotkey_manager.register(hotkey) {
            error!("Failed to register hotkey Alt+{}: {:?}", i, e);
        } else {
            hotkeys.push((hotkey.id(), i as u32));
        }
    }
    info!("Registered {} hotkeys", hotkeys.len());

    // Create Tray Menu
    let tray_menu = Menu::new();
    let quit_item = MenuItem::new("닫기", true, None);
    tray_menu.append(&quit_item).unwrap();

    // Create Tray Icon
    info!("Creating tray icon...");
    let icon = tray_icon::Icon::from_resource(1, None)
        .or_else(|_| tray_icon::Icon::from_path("icon.ico", Some((32, 32))))
        .expect("Failed to load icon");

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("DeskHop")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");
    info!("Tray icon created");

    let menu_channel = MenuEvent::receiver();
    let hotkey_channel = GlobalHotKeyEvent::receiver();

    info!("Starting event loop...");
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => {
                    info!("Close requested from window");
                    elwt.exit();
                }
                winit::event::Event::AboutToWait => {
                    // Handle Tray Menu Events
                    if let Ok(event) = menu_channel.try_recv() {
                        debug!("Tray menu event: {:?}", event);
                        if event.id == quit_item.id() {
                            info!("Quit requested via tray menu (닫기)");
                            elwt.exit();
                        }
                    }

                    // Handle Hotkey Events
                    if let Ok(event) = hotkey_channel.try_recv() {
                        debug!("Hotkey event: {:?}", event);
                        for (id, index) in &hotkeys {
                            if event.id == *id {
                                info!("Switching to desktop {}", index);
                                desktop_manager.switch_to(*index - 1);
                                osd_window.update_text(&format!("Desktop {}", index));
                            }
                        }
                    }
                }
                _ => (),
            }
        })
        .unwrap();
}
