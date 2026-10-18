//! 문제 진단용 로그. 하루에 파일 하나(g-routine-YYYY-MM-DD.log)를 쓰고 7일이 지나면 지운다.
//! 시작 직후에는 앱 폴더(AppData)에 쓰다가, 데이터 폴더가 준비되면 그 안의 logs로 옮겨 쓴다.
//! (복원 프로그램이 C드라이브를 지워도 D드라이브의 로그는 남는다)
//! 루틴 이름 같은 개인 내용은 남기지 않는다.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{Duration, Local, NaiveDate};
use log::{Level, LevelFilter, Log, Metadata, Record};

const PREFIX: &str = "g-routine-";
const KEEP_DAYS: i64 = 7;
/// 하루 파일이 이보다 커지면 그날은 더 쓰지 않는다 (무언가 반복해서 실패할 때 디스크를 채우지 않게)
const MAX_FILE_BYTES: u64 = 1024 * 1024;

struct Sink {
    dir: PathBuf,
    day: NaiveDate,
    file: Option<File>,
    written: u64,
    capped: bool,
}

pub struct FileLogger {
    sink: Mutex<Option<Sink>>,
}

static LOGGER: FileLogger = FileLogger { sink: Mutex::new(None) };

fn file_name(day: NaiveDate) -> String {
    format!("{PREFIX}{}.log", day.format("%Y-%m-%d"))
}

fn open_for(dir: &Path, day: NaiveDate) -> (Option<File>, u64) {
    if fs::create_dir_all(dir).is_err() {
        return (None, 0);
    }
    let path = dir.join(file_name(day));
    let written = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    (OpenOptions::new().create(true).append(true).open(path).ok(), written)
}

/// 보관 기간이 지난 로그 파일을 지운다
fn remove_old(dir: &Path, today: NaiveDate) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let oldest = today - Duration::days(KEEP_DAYS - 1);
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let day = name
            .strip_prefix(PREFIX)
            .and_then(|s| s.strip_suffix(".log"))
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        if day.is_some_and(|d| d < oldest) {
            let _ = fs::remove_file(e.path());
        }
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // 다른 라이브러리의 자잘한 정보는 빼고, 경고 이상만 받는다
        metadata.target().starts_with("g_routine") || metadata.level() <= Level::Warn
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let now = Local::now();
        let line = format!(
            "{} {:<5} [{}] {}\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target().trim_start_matches("g_routine_lib::"),
            record.args()
        );
        let Ok(mut guard) = self.sink.lock() else { return };
        let Some(sink) = guard.as_mut() else { return };
        let today = now.date_naive();
        if sink.day != today {
            let (file, written) = open_for(&sink.dir, today);
            remove_old(&sink.dir, today);
            *sink = Sink { dir: sink.dir.clone(), day: today, file, written, capped: false };
        }
        if sink.capped {
            return;
        }
        if let Some(f) = sink.file.as_mut() {
            if sink.written + line.len() as u64 > MAX_FILE_BYTES {
                let _ = f.write_all("로그가 너무 커서 오늘은 더 남기지 않아요\n".as_bytes());
                sink.capped = true;
                return;
            }
            if f.write_all(line.as_bytes()).is_ok() {
                sink.written += line.len() as u64;
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.sink.lock() {
            if let Some(f) = guard.as_mut().and_then(|s| s.file.as_mut()) {
                let _ = f.flush();
            }
        }
    }
}

/// 로그를 켠다 (앱 시작 때 한 번). 앱이 죽는 오류(panic)도 로그에 남긴다.
pub fn init(dir: PathBuf) {
    use_dir(dir);
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("앱이 예상치 못하게 멈췄어요: {info}");
        LOGGER.flush();
        default_hook(info);
    }));
}

/// 로그를 쓸 폴더를 바꾼다 (데이터 폴더가 준비되거나 바뀌었을 때)
pub fn use_dir(dir: PathBuf) {
    let today = Local::now().date_naive();
    if let Ok(mut guard) = LOGGER.sink.lock() {
        if guard.as_ref().is_some_and(|s| s.dir == dir) {
            return;
        }
        let (file, written) = open_for(&dir, today);
        remove_old(&dir, today);
        *guard = Some(Sink { dir, day: today, file, written, capped: false });
    }
}

pub fn current_dir() -> Option<PathBuf> {
    LOGGER.sink.lock().ok().and_then(|g| g.as_ref().map(|s| s.dir.clone()))
}

/// 최근 로그를 오래된 것부터 최대 max_lines줄 (문제 보고용)
pub fn recent_lines(dir: &Path, max_lines: usize) -> Vec<String> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(PREFIX)))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut lines: Vec<String> = Vec::new();
    for f in files {
        if let Ok(file) = File::open(&f) {
            lines.extend(BufReader::new(file).lines().map_while(Result::ok));
        }
    }
    let skip = lines.len().saturating_sub(max_lines);
    lines.split_off(skip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_log_files_are_removed_and_recent_lines_are_read_in_order() {
        let t = tempfile::tempdir().unwrap();
        let day = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        fs::write(t.path().join(file_name(day("2026-10-01"))), "아주 오래된 줄\n").unwrap();
        fs::write(t.path().join(file_name(day("2026-10-12"))), "첫째 줄\n둘째 줄\n").unwrap();
        fs::write(t.path().join(file_name(day("2026-10-18"))), "셋째 줄\n").unwrap();
        fs::write(t.path().join("다른 파일.txt"), "남겨 둔다").unwrap();

        remove_old(t.path(), day("2026-10-18"));
        assert!(!t.path().join(file_name(day("2026-10-01"))).exists());
        assert!(t.path().join(file_name(day("2026-10-12"))).exists());
        assert!(t.path().join("다른 파일.txt").exists());

        assert_eq!(recent_lines(t.path(), 2), vec!["둘째 줄", "셋째 줄"]);
        assert_eq!(recent_lines(t.path(), 10).len(), 3);
    }
}
