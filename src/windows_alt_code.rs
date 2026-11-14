use std::{cell::RefCell, rc::Rc};

use windows_sys::Win32::UI::WindowsAndMessaging::{
    MSG, WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use winit::platform::windows::EventLoopBuilderExtWindows;

#[derive(Default)]
struct Sequence {
    window: usize,
    digits: bool,
    released: bool,
}

impl Sequence {
    fn observe(&mut self, window: usize, message: u32, key: usize, flags: isize) -> Option<char> {
        const ALT: usize = 0x12;
        const EXTENDED: isize = 1 << 24;
        const ALT_CONTEXT: isize = 1 << 29;
        const REPEAT: isize = 1 << 30;
        match message {
            WM_KEYDOWN | WM_SYSKEYDOWN if key == ALT && flags & EXTENDED == 0 => {
                if flags & REPEAT == 0 {
                    *self = Self {
                        window,
                        ..Self::default()
                    };
                }
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if self.window == window
                    && !self.released
                    && flags & ALT_CONTEXT != 0
                    && (0x60..=0x69).contains(&key)
                {
                    self.digits = true;
                } else {
                    *self = Self::default();
                }
            }
            WM_KEYUP | WM_SYSKEYUP if key == ALT => {
                self.released = self.window == window && self.digits && flags & EXTENDED == 0;
            }
            WM_CHAR if self.released => {
                let same_window = self.window == window;
                *self = Self::default();
                if same_window {
                    return u32::try_from(key)
                        .ok()
                        .and_then(char::from_u32)
                        .filter(|ch| !ch.is_control());
                }
            }
            _ => {}
        }
        None
    }
}

#[derive(Clone, Default)]
pub struct AltCodeInput(Rc<RefCell<String>>);

impl AltCodeInput {
    pub fn hook(&self) -> eframe::EventLoopBuilderHook {
        let pending = self.0.clone();
        Box::new(move |builder| {
            let mut sequence = Sequence::default();
            builder.with_msg_hook(move |message| {
                // Winit supplies a valid MSG for the duration of the callback.
                let message = unsafe { &*message.cast::<MSG>() };
                let Some(ch) = sequence.observe(
                    message.hwnd as usize,
                    message.message,
                    message.wParam,
                    message.lParam,
                ) else {
                    return false;
                };
                let mut text = pending.borrow_mut();
                if text.len() < 256 {
                    text.push(ch);
                }
                // This commit follows key release, so winit cannot attach it to a key-down event.
                true
            });
        })
    }

    pub fn take(&self) -> String {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_the_windows_commit_once() {
        let mut sequence = Sequence::default();
        assert_eq!(sequence.observe(1, WM_CHAR, 97, 0), None);
        sequence.observe(1, WM_SYSKEYDOWN, 0x12, 0);
        for key in [0x62, 0x62, 0x67] {
            sequence.observe(1, WM_SYSKEYDOWN, key, 1 << 29);
        }
        sequence.observe(1, WM_SYSKEYUP, 0x12, 0);
        assert_eq!(sequence.observe(1, WM_CHAR, 'π' as usize, 0), Some('π'));
        assert_eq!(sequence.observe(1, WM_CHAR, 'π' as usize, 0), None);
    }

    #[test]
    fn preserves_shortcuts_and_other_windows() {
        for (flags, key, target) in [(0, 0x41, 1), (1 << 24, 0x62, 1), (0, 0x62, 2)] {
            let mut sequence = Sequence::default();
            sequence.observe(1, WM_SYSKEYDOWN, 0x12, flags);
            sequence.observe(1, WM_SYSKEYDOWN, key, 1 << 29);
            sequence.observe(1, WM_SYSKEYUP, 0x12, flags);
            assert_eq!(sequence.observe(target, WM_CHAR, 97, 0), None);
        }
    }
}
