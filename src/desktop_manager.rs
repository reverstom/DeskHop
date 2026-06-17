use winvd::*;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub struct DesktopManager;

impl DesktopManager {
    pub fn new() -> Self {
        Self
    }

    pub fn switch_to(&self, index: u32) {
        if let Err(e) = switch_desktop(index) {
            eprintln!("Failed to switch to desktop {}: {:?}", index, e);
        }
    }

    pub fn move_focused_window_to(&self, index: u32) {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                eprintln!("Failed to get foreground window");
                return;
            }

            if let Err(e) = move_window_to_desktop(index, &hwnd) {
                eprintln!("Failed to move window to desktop {}: {:?}", index, e);
            }
        }
    }

    // pub fn get_current_desktop(&self) -> u32 {
    //     get_current_desktop()
    //         .and_then(|d| d.get_index())
    //         .unwrap_or(0)
    // }

    // pub fn get_desktop_count(&self) -> u32 {
    //     get_desktop_count().unwrap_or(0)
    // }
}
