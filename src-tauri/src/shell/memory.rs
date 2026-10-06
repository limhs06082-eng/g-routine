//! 위젯이 숨겨져 트레이에만 있을 때 WebView2에 메모리를 줄이라고 알린다.
//! (WebView2 MemoryUsageTargetLevel: Low면 런타임이 캐시 등을 정리해 메모리를 덜 쓴다)

use tauri::WebviewWindow;

/// 위젯 표시 여부에 맞춰 WebView2 메모리 목표 수준을 바꾼다.
/// 실패해도 위젯 동작에는 영향이 없으므로 오류는 무시한다.
pub fn set_low_memory(window: &WebviewWindow, low: bool) {
    #[cfg(windows)]
    {
        let _ = window.with_webview(move |webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::{
                ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
            };
            use windows_core::Interface;

            let level = if low {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            };
            // SAFETY: Tauri가 넘겨준 살아 있는 WebView2 컨트롤러에서 COM 메서드를 호출할 뿐이다.
            unsafe {
                if let Ok(core) = webview.controller().CoreWebView2() {
                    if let Ok(core19) = core.cast::<ICoreWebView2_19>() {
                        let _ = core19.SetMemoryUsageTargetLevel(level);
                    }
                }
            }
        });
    }
    #[cfg(not(windows))]
    {
        let _ = (window, low);
    }
}
