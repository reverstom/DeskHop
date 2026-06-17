use log::{debug, info};
use std::ffi::c_void;
use std::sync::{Arc, Mutex, Once};
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::Input::KeyboardAndMouse::ReleaseCapture, Win32::UI::WindowsAndMessaging::*,
};

static REGISTER_CLASS: Once = Once::new();
const CLASS_NAME: PCWSTR = w!("DeskHopOSD");

pub struct OsdWindow {
    hwnd: HWND,
    text: Arc<Mutex<String>>,
}

impl OsdWindow {
    pub fn new(x: i32, y: i32, opacity: u8) -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let text = Arc::new(Mutex::new(String::from("Desktop 1")));

        REGISTER_CLASS.call_once(|| {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(Self::wnd_proc),
                hInstance: instance.into(),
                lpszClassName: CLASS_NAME,
                hbrBackground: unsafe { HBRUSH(GetStockObject(BLACK_BRUSH).0) },
                ..Default::default()
            };
            let _ = unsafe { RegisterClassW(&wc) };
        });

        // Removed WS_EX_TRANSPARENT to allow interaction as requested by logging requirements
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("DeskHop OSD"),
                WS_POPUP,
                x,
                y,
                160,
                50,
                None,
                None,
                instance,
                Some(Box::into_raw(Box::new(text.clone())) as *mut c_void),
            )?
        };

        let osd = Self { hwnd, text };
        osd.set_opacity(opacity);

        Ok(osd)
    }

    pub fn show(&self) {
        debug!("Showing OSD Window");
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
        };
    }

    pub fn set_opacity(&self, opacity: u8) {
        debug!("Setting OSD opacity to {}", opacity);
        unsafe {
            let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), opacity, LWA_ALPHA);
        }
    }

    pub fn update_text(&self, text: &str) {
        debug!("Updating OSD text to: {}", text);
        {
            let mut t = self.text.lock().unwrap();
            *t = text.to_string();
        }
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, true);
        };
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_NCCREATE => {
                let createstruct = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
                unsafe {
                    SetWindowLongPtrW(hwnd, GWL_USERDATA, createstruct.lpCreateParams as isize)
                };
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
            WM_LBUTTONDOWN => {
                debug!("OSD Window drag started (Left Button)");
                unsafe {
                    let _ = ReleaseCapture();
                    let _ =
                        PostMessageW(hwnd, WM_SYSCOMMAND, WPARAM(SC_MOVE as usize + 2), LPARAM(0));
                }
                LRESULT(0)
            }
            WM_EXITSIZEMOVE => {
                let mut rect = RECT::default();
                unsafe {
                    let _ = GetWindowRect(hwnd, &mut rect);
                }
                debug!("OSD Window move finished at ({}, {})", rect.left, rect.top);
                crate::settings::update_location(rect.left, rect.top);
                LRESULT(0)
            }
            WM_RBUTTONUP => {
                debug!("OSD Window right-clicked - Showing context menu");
                let mut pt = POINT::default();
                let _ = unsafe { GetCursorPos(&mut pt) };
                let hmenu = unsafe { CreatePopupMenu().unwrap() };

                // 현재 설정된 투명도 값을 가져옵니다.
                let current_opacity = crate::settings::load_settings().opacity;

                unsafe {
                    for i in 1..=10 {
                        let percentage = i * 10;
                        let item_opacity = (255.0 * (percentage as f32 / 100.0)) as u8;

                        let mut flags = MF_STRING;
                        // 현재 투명도와 메뉴 항목의 투명도가 일치하거나 가장 근접한 경우 체크 표시를 합니다.
                        if (item_opacity as i16 - current_opacity as i16).abs() < 13 {
                            flags |= MF_CHECKED;
                        }

                        let label = format!("{}%\0", percentage);
                        let wide_label: Vec<u16> = label.encode_utf16().collect();
                        let _ = AppendMenuW(
                            hmenu,
                            flags,
                            1000 + i as usize,
                            PCWSTR(wide_label.as_ptr()),
                        );
                    }

                    // 투명도 항목 아래에 구분선(Separator) 추가
                    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

                    // "닫기" 메뉴 항목 추가 (ID: 1011)
                    let close_label: Vec<u16> = "닫기\0".encode_utf16().collect();
                    let _ = AppendMenuW(hmenu, MF_STRING, 1011, PCWSTR(close_label.as_ptr()));

                    let _ = TrackPopupMenu(hmenu, TPM_LEFTALIGN, pt.x, pt.y, 0, hwnd, None);
                    let _ = DestroyMenu(hmenu);
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let id = wparam.0 as u16;
                debug!("OSD Window command received: {}", id);

                // "닫기" 메뉴 항목 처리
                if id == 1011 {
                    info!("Quit requested via OSD menu (전체 종료)");
                    std::process::exit(0);
                }

                let opacity = if (1001..=1010).contains(&id) {
                    let percentage = (id - 1000) as f32 * 10.0;
                    info!("Changing opacity to {}%", percentage);
                    (255.0 * (percentage / 100.0)) as u8
                } else {
                    return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
                };
                unsafe {
                    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), opacity, LWA_ALPHA);
                }
                crate::settings::update_opacity(opacity);
                LRESULT(0)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                unsafe {
                    let hdc = BeginPaint(hwnd, &mut ps);
                    let mut rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rect);

                    // Draw Background
                    let brush = CreateSolidBrush(COLORREF(0x000000)); // Black
                    let _ = FillRect(hdc, &rect, brush);
                    let _ = DeleteObject(brush);

                    // Draw Border
                    let border_brush = CreateSolidBrush(COLORREF(0xFFFFFF)); // White
                    let _ = FrameRect(hdc, &rect, border_brush);
                    let _ = DeleteObject(border_brush);

                    SetBkMode(hdc, TRANSPARENT);
                    SetTextColor(hdc, COLORREF(0x0000FFFF)); // Yellow (RGB: 255, 255, 0)

                    // 텍스트 색상이 올바르게 적용되도록 시스템 폰트를 선택합니다.
                    let hfont_stock = GetStockObject(SYSTEM_FONT);
                    let old_font = SelectObject(hdc, hfont_stock);

                    let userdata = GetWindowLongPtrW(hwnd, GWL_USERDATA);
                    if userdata != 0 {
                        let text_ptr = userdata as *mut Arc<Mutex<String>>;
                        if let Ok(text_guard) = { (*text_ptr).lock() } {
                            let mut wide_text: Vec<u16> = text_guard.encode_utf16().collect();

                            DrawTextW(
                                hdc,
                                &mut wide_text,
                                &mut rect,
                                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                            );
                        }
                    }

                    // 원래 폰트를 복원합니다.
                    SelectObject(hdc, old_font);

                    let _ = EndPaint(hwnd, &ps);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                debug!("OSD Window being destroyed");
                let userdata = unsafe { SetWindowLongPtrW(hwnd, GWL_USERDATA, 0) };
                if userdata != 0 {
                    unsafe {
                        let _ = Box::from_raw(userdata as *mut Arc<Mutex<String>>);
                    };
                }
                std::process::exit(0);
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
