use std::ptr::{NonNull, null_mut};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::Keyboard_And_Mouse::*;
use windows::Win32::UI::Input::Keyboard_And_Mouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub trait EventPoster: Send + Sync {
    fn post_backspace(&self, count: usize) -> Result<(), String>;
    fn post_unicode(&self, text: &str) -> Result<(), String>;
}

pub struct WinEventPoster;

impl EventPoster for WinEventPoster {
    fn post_backspace(&self, count: usize) -> Result<(), String> {
        let mut inputs = Vec::new();
        for _ in 0..count {
            inputs.push(create_key_input(VK_BACK, false));
            inputs.push(create_key_input(VK_BACK, true));
        }
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        Ok(())
    }

    fn post_unicode(&self, text: &str) -> Result<(), String> {
        for c in text.encode_utf16() {
            let mut inputs = [
                create_unicode_input(c, false),
                create_unicode_input(c, true),
            ];
            unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        }
        Ok(())
    }
}

// Helper functions cho Windows Input
fn create_key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 999, // Magic number để nhận diện event do chính mình tạo ra
            },
        },
    }
}

fn create_unicode_input(c: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: c,
                dwFlags: if up {
                    KEYEVENTF_KEYUP | KEYEVENTF_UNICODE
                } else {
                    KEYEVENTF_UNICODE
                },
                time: 0,
                dwExtraInfo: 999,
            },
        },
    }
}

pub struct WindowsOS<'def, P: EventPoster> {
    buffer: &'def Mutex<IncrementalBuffer<'static>>, // Giả định IncrementalBuffer đã có
    raw_tracker: &'def Mutex<String>,
    spoof_until: &'def Mutex<Option<Instant>>,
    poster: P,
}

static mut GLOBAL_STATE: Option<*mut c_void> = None;

impl<'def, P: EventPoster> WindowsOS<'def, P> {
    pub fn new(
        buffer: &'def Mutex<IncrementalBuffer<'static>>,
        raw_tracker: &'def Mutex<String>,
        spoof_until: &'def Mutex<Option<Instant>>,
        poster: P,
    ) -> Self {
        Self {
            buffer,
            raw_tracker,
            spoof_until,
            poster,
        }
    }

    fn is_in_spoof_window(&self) -> bool {
        let spoof = self.spoof_until.lock().unwrap();
        matches!(*spoof, Some(until) if Instant::now() <= until)
    }

    fn handle_key_down(&mut self, vk: VIRTUAL_KEY, shift: bool, caps: bool) -> LRESULT {
        if should_clear_tracker_win(vk) {
            self.clear_tracker();
            return LRESULT(0); // Pass
        }

        if vk == VK_BACK {
            let mut tracker = self.raw_tracker.lock().unwrap();
            tracker.pop();
            return LRESULT(0);
        }

        if let Some(ch) = win_vk_to_char(vk, shift, caps) {
            let old_raw = self.raw_tracker.lock().unwrap().clone();
            {
                self.raw_tracker.lock().unwrap().push(ch);
            }

            let current_raw = self.raw_tracker.lock().unwrap().clone();
            let transformed = self.handle_transform(&current_raw);

            if current_raw != transformed {
                let to_delete = self.handle_transform(&old_raw).chars().count();
                self.apply_transformation(to_delete, &transformed);
                return LRESULT(1); // Block
            }
        }
        LRESULT(0)
    }

    fn apply_transformation(&mut self, count: usize, text: &str) {
        let until = Instant::now() + Duration::from_millis(50);
        *self.spoof_until.lock().unwrap() = Some(until);

        let _ = self.poster.post_backspace(count);
        let _ = self.poster.post_unicode(text);
    }

    fn handle_transform(&self, input: &str) -> String {
        let mut buffer = self.buffer.lock().unwrap();
        for c in input.chars() {
            buffer.push(c);
        }
        let res = buffer.view().to_string();
        buffer.clear();
        res
    }

    fn clear_tracker(&self) {
        self.raw_tracker.lock().unwrap().clear();
    }

    pub fn run_loop<P: EventPoster + 'static>(state: *mut WindowsOS<'static, P>) {
        unsafe {
            GLOBAL_STATE = Some(state as *mut _);
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0).unwrap();
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hook);
        }
    }
}

// Hook Callback
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && (wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN) {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);

        // Tránh loop vô tận: Nếu event có extra info là 999 (do mình post) thì bỏ qua
        if kbd.dwExtraInfo == 999 {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        if let Some(ptr) = GLOBAL_STATE {
            let state = &mut *(ptr as *mut WindowsOS<WinEventPoster>);

            if state.is_in_spoof_window() {
                return CallNextHookEx(None, code, wparam, lparam);
            }

            let vk = VIRTUAL_KEY(kbd.vkCode as u16);

            // Logic Toggle (giống Cmd+Shift+Z bên Mac -> Ctrl+Shift+Z)
            let ctrl = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
            let shift = GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
            if vk == VK_Z && ctrl && shift {
                // dispatch_toggle_engine();
                return LRESULT(1);
            }

            let caps = (GetKeyState(VK_CAPITAL.0 as i32) & 1) != 0;

            if state.handle_key_down(vk, shift, caps) == LRESULT(1) {
                return LRESULT(1); // Block phím nguyên bản
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn win_vk_to_char(vk: VIRTUAL_KEY, shift: bool, caps_lock: bool) -> Option<char> {
    let uppercase = shift ^ caps_lock;

    match vk {
        // Chữ cái (Alpha) - Áp dụng logic XOR cho Caps Lock và Shift
        VK_A => Some(if uppercase { 'A' } else { 'a' }),
        VK_B => Some(if uppercase { 'B' } else { 'b' }),
        VK_C => Some(if uppercase { 'C' } else { 'c' }),
        VK_D => Some(if uppercase { 'D' } else { 'd' }),
        VK_E => Some(if uppercase { 'E' } else { 'e' }),
        VK_F => Some(if uppercase { 'F' } else { 'f' }),
        VK_G => Some(if uppercase { 'G' } else { 'g' }),
        VK_H => Some(if uppercase { 'H' } else { 'h' }),
        VK_I => Some(if uppercase { 'I' } else { 'i' }),
        VK_J => Some(if uppercase { 'J' } else { 'j' }),
        VK_K => Some(if uppercase { 'K' } else { 'k' }),
        VK_L => Some(if uppercase { 'L' } else { 'l' }),
        VK_M => Some(if uppercase { 'M' } else { 'm' }),
        VK_N => Some(if uppercase { 'N' } else { 'n' }),
        VK_O => Some(if uppercase { 'O' } else { 'o' }),
        VK_P => Some(if uppercase { 'P' } else { 'p' }),
        VK_Q => Some(if uppercase { 'Q' } else { 'q' }),
        VK_R => Some(if uppercase { 'R' } else { 'r' }),
        VK_S => Some(if uppercase { 'S' } else { 's' }),
        VK_T => Some(if uppercase { 'T' } else { 't' }),
        VK_U => Some(if uppercase { 'U' } else { 'u' }),
        VK_V => Some(if uppercase { 'V' } else { 'v' }),
        VK_W => Some(if uppercase { 'W' } else { 'w' }),
        VK_X => Some(if uppercase { 'X' } else { 'x' }),
        VK_Y => Some(if uppercase { 'Y' } else { 'y' }),
        VK_Z => Some(if uppercase { 'Z' } else { 'z' }),

        // Hàng phím số (Numbers)
        VK_0 => Some(if shift { ')' } else { '0' }),
        VK_1 => Some(if shift { '!' } else { '1' }),
        VK_2 => Some(if shift { '@' } else { '2' }),
        VK_3 => Some(if shift { '#' } else { '3' }),
        VK_4 => Some(if shift { '$' } else { '4' }),
        VK_5 => Some(if shift { '%' } else { '5' }),
        VK_6 => Some(if shift { '^' } else { '6' }),
        VK_7 => Some(if shift { '&' } else { '7' }),
        VK_8 => Some(if shift { '*' } else { '8' }),
        VK_9 => Some(if shift { '(' } else { '9' }),

        // Ký tự đặc biệt (Symbols) - Tên VK trên Windows hơi khác Mac một chút
        VK_OEM_MINUS => Some(if shift { '_' } else { '-' }),
        VK_OEM_PLUS => Some(if shift { '+' } else { '=' }),
        VK_OEM_4 => Some(if shift { '{' } else { '[' }), // Left Bracket
        VK_OEM_6 => Some(if shift { '}' } else { ']' }), // Right Bracket
        VK_OEM_5 => Some(if shift { '|' } else { '\\' }), // Backslash
        VK_OEM_1 => Some(if shift { ':' } else { ';' }), // Semicolon
        VK_OEM_7 => Some(if shift { '"' } else { '\'' }), // Quote
        VK_OEM_COMMA => Some(if shift { '<' } else { ',' }),
        VK_OEM_PERIOD => Some(if shift { '>' } else { '.' }),
        VK_OEM_2 => Some(if shift { '?' } else { '/' }), // Slash
        VK_OEM_3 => Some(if shift { '~' } else { '`' }), // Backtick/Grave

        // Phím chức năng cơ bản
        VK_SPACE => Some(' '),
        VK_RETURN => Some('\n'),
        VK_TAB => Some('\t'),

        _ => None,
    }
}

fn should_clear_tracker_win(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_RETURN | VK_TAB | VK_ESCAPE | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN
    )
}
