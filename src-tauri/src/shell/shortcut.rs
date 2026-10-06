//! 전역 단축키 Ctrl+Alt+G: 위젯을 숨기거나 다시 띄운다.
//! 수업 중 TV · 프로젝터로 화면을 띄울 때 위젯을 바로 감추려는 용도다.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::shell::window;

pub fn widget_toggle() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyG)
}

/// 단축키를 등록한다. 다른 프로그램이 이미 쓰고 있으면 false (앱은 그대로 동작한다).
pub fn register(app: &AppHandle) -> bool {
    app.global_shortcut()
        .on_shortcut(widget_toggle(), |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                window::toggle_widget(app);
            }
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_is_ctrl_alt_g() {
        assert!(widget_toggle().matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyG));
        assert!(!widget_toggle().matches(Modifiers::CONTROL, Code::KeyG));
    }
}
