//! 백그라운드 작업을 스레드 하나에서 차례로 돌린다.
//! (날짜 감시 · 마감 알림 · 공휴일 표 · 두 번째 백업처럼 가끔 깨어나 잠깐 일하는 것들)
//! 자동 업데이트는 설치 파일을 받는 동안 오래 걸릴 수 있어 따로 둔다 (shell::update).

use std::time::{Duration, Instant};

use tauri::AppHandle;

pub struct Job {
    pub name: &'static str,
    /// 앱이 켜진 뒤 처음 돌기까지 기다리는 시간
    pub first: Duration,
    pub every: Duration,
    pub run: Box<dyn FnMut(&AppHandle) + Send>,
}

impl Job {
    pub fn new(name: &'static str, first: Duration, every: Duration, run: impl FnMut(&AppHandle) + Send + 'static) -> Self {
        Job { name, first, every, run: Box::new(run) }
    }
}

/// 다음에 돌 작업과 그때까지 기다릴 시간
fn next_due(schedule: &[Instant], now: Instant) -> (usize, Duration) {
    let (i, at) = schedule.iter().enumerate().min_by_key(|(_, at)| **at).expect("작업이 하나 이상 있어야 한다");
    (i, at.saturating_duration_since(now))
}

pub fn spawn(app: AppHandle, mut jobs: Vec<Job>) {
    if jobs.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        let start = Instant::now();
        let mut schedule: Vec<Instant> = jobs.iter().map(|j| start + j.first).collect();
        loop {
            let (i, wait) = next_due(&schedule, Instant::now());
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
            let job = &mut jobs[i];
            let started = Instant::now();
            (job.run)(&app);
            if started.elapsed() > Duration::from_secs(10) {
                log::info!("백그라운드 작업 '{}'에 {}초 걸렸어요", job.name, started.elapsed().as_secs());
            }
            schedule[i] = Instant::now() + job.every;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_earliest_job_runs_next() {
        let now = Instant::now();
        let schedule = [now + Duration::from_secs(30), now + Duration::from_secs(5), now + Duration::from_secs(120)];
        assert_eq!(next_due(&schedule, now), (1, Duration::from_secs(5)));
        // 이미 지난 작업은 기다리지 않는다
        let late = [now + Duration::from_secs(10), now];
        assert_eq!(next_due(&late, now + Duration::from_secs(3)), (1, Duration::ZERO));
    }
}
