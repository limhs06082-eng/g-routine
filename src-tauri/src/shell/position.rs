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

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };

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
}
