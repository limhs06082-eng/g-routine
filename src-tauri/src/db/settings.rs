use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::day::parse_day;
use crate::error::{AppError, AppResult};
use crate::model::Settings;

pub const THEMES: [&str; 5] = ["lavender", "mint", "peach", "sky", "lemon"];
const BOOL_KEYS: [&str; 6] = ["always_on_top", "autostart", "hide_weekends", "due_alerts", "hide_holidays", "mini_mode"];

pub fn get(c: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(c.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}

pub fn set(c: &Connection, key: &str, value: &str) -> AppResult<()> {
    c.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn remove(c: &Connection, key: &str) -> AppResult<()> {
    c.execute("DELETE FROM settings WHERE key = ?1", [key])?;
    Ok(())
}

fn get_bool(c: &Connection, key: &str, default: bool) -> AppResult<bool> {
    Ok(get(c, key)?.map(|v| v == "true").unwrap_or(default))
}

pub fn load(c: &Connection) -> AppResult<Settings> {
    Ok(Settings {
        always_on_top: get_bool(c, "always_on_top", true)?,
        autostart: get_bool(c, "autostart", true)?,
        hide_weekends: get_bool(c, "hide_weekends", true)?,
        due_alerts: get_bool(c, "due_alerts", true)?,
        hide_holidays: get_bool(c, "hide_holidays", true)?,
        mini_mode: get_bool(c, "mini_mode", false)?,
        vacation_start: get(c, "vacation_start")?.filter(|d| parse_day(d).is_some()),
        vacation_end: get(c, "vacation_end")?.filter(|d| parse_day(d).is_some()),
        day_start_hour: get(c, "day_start_hour")?
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|h| *h < 24)
            .unwrap_or(4),
        seen_version: get(c, SEEN_VERSION)?.filter(|v| is_version(v)),
        theme: get(c, "theme")?
            .filter(|t| THEMES.contains(&t.as_str()))
            .unwrap_or_else(|| "lavender".into()),
    })
}

/// '바뀐 점' 안내를 마지막으로 본 앱 버전. 새로 설치할 때는 그 버전을 적어 두어 안내를 띄우지 않는다.
pub const SEEN_VERSION: &str = "seen_version";

/// 0.3.0 같은 버전 문자열이면 (주, 부, 수) 숫자로
fn version_parts(v: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 || !parts.iter().all(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
}

fn is_version(v: &str) -> bool {
    version_parts(v).is_some()
}

/// 사용자가 바꿀 수 있는 설정만 검증 후 저장한다.
pub fn apply(c: &Connection, key: &str, value: &str) -> AppResult<()> {
    if key == SEEN_VERSION {
        return apply_seen_version(c, value);
    }
    let valid = if BOOL_KEYS.contains(&key) {
        value == "true" || value == "false"
    } else if key == "day_start_hour" {
        value.parse::<u32>().map(|h| h < 24).unwrap_or(false)
    } else if key == "theme" {
        THEMES.contains(&value)
    } else {
        return Err(AppError::invalid("알 수 없는 설정이에요"));
    };
    if !valid {
        return Err(AppError::invalid("설정 값이 올바르지 않아요"));
    }
    set(c, key, value)
}

/// 본 버전은 앞으로만 간다. 복원 프로그램이 C드라이브를 되돌려 예전 버전이 잠깐 실행돼도
/// 더 새 버전의 기록을 덮어쓰지 않아야, 업데이트 뒤 '바뀐 점'이 날마다 다시 뜨지 않는다.
fn apply_seen_version(c: &Connection, value: &str) -> AppResult<()> {
    let next = version_parts(value).ok_or_else(|| AppError::invalid("설정 값이 올바르지 않아요"))?;
    let current = get(c, SEEN_VERSION)?.as_deref().and_then(version_parts);
    if current.is_some_and(|cur| cur >= next) {
        return Ok(());
    }
    set(c, SEEN_VERSION, value)
}

pub fn window_pos(c: &Connection) -> AppResult<Option<(i32, i32)>> {
    let x = get(c, "window_x")?.and_then(|v| v.parse().ok());
    let y = get(c, "window_bottom")?.and_then(|v| v.parse().ok());
    Ok(x.zip(y))
}

pub fn set_window_pos(c: &Connection, x: i32, y: i32) -> AppResult<()> {
    let tx = c.unchecked_transaction()?;
    let x_str = x.to_string();
    let y_str = y.to_string();
    tx.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["window_x", x_str],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["window_bottom", y_str],
    )?;
    tx.commit()?;
    Ok(())
}

/// 방학 기간을 함께 저장하거나(Some) 함께 지운다(None). 검증은 호출하는 쪽(service)이 한다.
pub fn set_vacation(c: &Connection, range: Option<(&str, &str)>) -> AppResult<()> {
    let tx = c.unchecked_transaction()?;
    match range {
        Some((start, end)) => {
            set(&tx, "vacation_start", start)?;
            set(&tx, "vacation_end", end)?;
        }
        None => {
            tx.execute("DELETE FROM settings WHERE key IN ('vacation_start', 'vacation_end')", [])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn clear_window_pos(c: &Connection) -> AppResult<()> {
    let tx = c.unchecked_transaction()?;
    tx.execute("DELETE FROM settings WHERE key = ?1", ["window_x"])?;
    tx.execute("DELETE FROM settings WHERE key = ?1", ["window_bottom"])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn defaults_when_empty() {
        let c = open_in_memory().unwrap();
        let s = load(&c).unwrap();
        assert_eq!(
            s,
            Settings {
                always_on_top: true,
                autostart: true,
                hide_weekends: true,
                day_start_hour: 4,
                theme: "lavender".into(),
                due_alerts: true,
                hide_holidays: true,
                mini_mode: false,
                vacation_start: None,
                vacation_end: None,
                seen_version: None,
            }
        );
    }

    #[test]
    fn apply_validates_and_persists() {
        let c = open_in_memory().unwrap();
        apply(&c, "hide_weekends", "false").unwrap();
        apply(&c, "day_start_hour", "5").unwrap();
        apply(&c, "theme", "mint").unwrap();
        apply(&c, "due_alerts", "false").unwrap();
        let s = load(&c).unwrap();
        assert_eq!((s.hide_weekends, s.day_start_hour, s.theme.as_str()), (false, 5, "mint"));
        assert!(!s.due_alerts);

        assert!(apply(&c, "hide_weekends", "yes").is_err());
        assert!(apply(&c, "day_start_hour", "24").is_err());
        assert!(apply(&c, "theme", "black").is_err());
        assert!(apply(&c, "window_x", "10").is_err());
    }

    #[test]
    fn seen_version_accepts_only_plain_versions() {
        let c = open_in_memory().unwrap();
        assert_eq!(load(&c).unwrap().seen_version, None);
        apply(&c, SEEN_VERSION, "0.3.0").unwrap();
        assert_eq!(load(&c).unwrap().seen_version.as_deref(), Some("0.3.0"));
        for bad in ["0.3", "v0.3.0", "0.3.0-beta", "0..1", "1.2.3.4", ""] {
            assert!(apply(&c, SEEN_VERSION, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn seen_version_never_goes_back_when_an_older_app_runs() {
        let c = open_in_memory().unwrap();
        apply(&c, SEEN_VERSION, "0.4.0").unwrap();
        // 복원 프로그램으로 0.3.0이 잠깐 실행돼 자기 버전을 적으려 해도
        apply(&c, SEEN_VERSION, "0.3.0").unwrap();
        assert_eq!(load(&c).unwrap().seen_version.as_deref(), Some("0.4.0"));
        apply(&c, SEEN_VERSION, "0.10.0").unwrap();
        assert_eq!(load(&c).unwrap().seen_version.as_deref(), Some("0.10.0"));
    }

    #[test]
    fn window_position_round_trip() {
        let c = open_in_memory().unwrap();
        assert_eq!(window_pos(&c).unwrap(), None);
        set_window_pos(&c, 1600, -20).unwrap();
        assert_eq!(window_pos(&c).unwrap(), Some((1600, -20)));
        clear_window_pos(&c).unwrap();
        assert_eq!(window_pos(&c).unwrap(), None);
    }
}
