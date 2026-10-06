#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// 작업 영역(작업표시줄 제외) 우측 하단에 margin을 두고 배치할 좌상단 좌표
pub fn bottom_right(work: Rect, w: i32, h: i32, margin: i32) -> (i32, i32) {
    (work.x + work.w - w - margin, work.y + work.h - h - margin)
}

/// 창의 중심이 어느 모니터 안에 있으면 보이는 위치로 본다.
pub fn center_inside(x: i32, y: i32, w: i32, h: i32, monitors: &[Rect]) -> bool {
    let (cx, cy) = (x + w / 2, y + h / 2);
    monitors.iter().any(|m| cx >= m.x && cx < m.x + m.w && cy >= m.y && cy < m.y + m.h)
}

/// 아래 모서리를 고정한 채 높이를 바꿀 때의 새 y
pub fn anchored_y(y: i32, old_h: i32, new_h: i32) -> i32 {
    y + old_h - new_h
}

/// 내용에 맞춘 새 높이와 y. 아래 모서리를 고정하고, 작업 영역 높이(위아래 여백 제외)까지만 늘린다.
/// 위로 늘리다 작업 영역 위쪽에 닿으면 위에 붙인다.
pub fn fit_height(y: i32, old_h: i32, wanted_h: i32, min_h: i32, work: Rect, margin: i32) -> (i32, i32) {
    let max_h = (work.h - 2 * margin).max(min_h);
    let h = wanted_h.clamp(min_h, max_h);
    let top = work.y + margin;
    (anchored_y(y, old_h, h).max(top), h)
}

/// 창 크기(w, h)에 맞는 좌상단 좌표. 저장 위치(x, 아래쪽 y)가 화면 안이면 그것을, 아니면 작업 영역 우측 하단을 쓴다.
/// 모니터 이동으로 DPI가 바뀌어 창 크기가 달라지면 새 크기로 다시 호출한다.
pub fn compute_position(
    saved: Option<(i32, i32)>,
    w: i32,
    h: i32,
    monitors: &[Rect],
    work: Rect,
    margin: i32,
) -> (i32, i32) {
    match saved {
        Some((x, bottom)) if center_inside(x, bottom - h, w, h, monitors) => (x, bottom - h),
        _ => bottom_right(work, w, h, margin),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };
    const WORK: Rect = Rect { x: 0, y: 0, w: 1920, h: 1032 }; // 작업표시줄 48px

    #[test]
    fn fit_height_grows_upward_keeping_bottom() {
        assert_eq!(fit_height(600, 300, 400, 100, WORK, 12), (500, 400));
        assert_eq!(fit_height(500, 400, 250, 100, WORK, 12), (650, 250));
    }

    #[test]
    fn fit_height_respects_minimum() {
        assert_eq!(fit_height(600, 300, 40, 100, WORK, 12), (800, 100));
    }

    #[test]
    fn fit_height_caps_at_work_area_and_stops_at_top() {
        // 작업 영역 1032 - 여백 12*2 = 1008이 최대 높이
        assert_eq!(fit_height(620, 400, 1500, 100, WORK, 12), (12, 1008));
        // 위로 늘리다 화면 위쪽에 닿으면 위에 붙이고 아래로 늘어난다
        assert_eq!(fit_height(100, 300, 600, 100, WORK, 12), (12, 600));
    }

    #[test]
    fn bottom_right_respects_work_area_and_margin() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 }; // 작업표시줄 48px
        assert_eq!(bottom_right(work, 280, 400, 12), (1628, 620));
        let second = Rect { x: 1920, y: 0, w: 2560, h: 1400 };
        assert_eq!(bottom_right(second, 350, 500, 15), (4115, 885));
    }

    #[test]
    fn visibility_uses_window_center() {
        assert!(center_inside(1600, 600, 280, 400, &[FHD]));
        assert!(!center_inside(1800, 600, 280, 400, &[FHD]));
        assert!(center_inside(1800, 600, 280, 400, &[FHD, Rect { x: 1920, y: 0, w: 1920, h: 1080 }]));
        assert!(!center_inside(100, 100, 280, 400, &[]));
    }

    #[test]
    fn anchored_resize_keeps_bottom_edge() {
        assert_eq!(anchored_y(600, 400, 300), 700);
        assert_eq!(anchored_y(600, 300, 450), 450);
    }

    #[test]
    fn recompute_after_dpi_rescale_keeps_edges_inside_work_area() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        let margin = 18; // 150% 배율
        // 100% 모니터 기준 크기로 먼저 배치
        let (x0, y0) = compute_position(None, 280, 400, &[FHD], work, margin);
        assert_eq!((x0 + 280, y0 + 400), (1920 - margin, 1032 - margin));
        // 150% 모니터로 옮겨져 창이 420x600으로 커진 뒤 다시 계산
        let (x1, y1) = compute_position(None, 420, 600, &[FHD], work, margin);
        assert_eq!((x1 + 420, y1 + 600), (1920 - margin, 1032 - margin));
        assert!(x1 >= work.x && y1 >= work.y);
    }

    #[test]
    fn saved_position_keeps_bottom_after_resize_or_falls_back() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        // 저장된 아래쪽 y를 유지한 채 높이만 바뀐다
        assert_eq!(compute_position(Some((1500, 900)), 420, 600, &[FHD], work, 18), (1500, 300));
        // 커진 크기에서는 중심이 화면 밖이면 기본 위치로
        assert_eq!(compute_position(Some((1900, 900)), 420, 600, &[FHD], work, 18), (1482, 414));
    }
}
