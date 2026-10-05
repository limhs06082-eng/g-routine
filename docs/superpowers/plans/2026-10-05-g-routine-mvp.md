# G-routine MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 교사용 매일 루틴 위젯 G-routine의 1단계(MVP, 스펙 F1~F18)를 Windows 데스크톱 앱으로 구현하고, 설치 파일과 포터블 exe까지 빌드한다.

**Architecture:** Tauri 2 앱. 모든 규칙(업무일 · 반복 · 스냅샷)과 저장은 Rust(`domain`, `db`, `service`, `storage`)에 두고 단위 테스트로 고정한다. React 화면은 `invoke()`로 결과만 받아 그린다. 창은 두 개다. 항상 떠 있는 `widget`(투명 · 프레임 없음)과 필요할 때만 여는 `manager`로 나뉜다. 데이터가 바뀌면 Rust가 `data-changed` 이벤트를 보내 두 창이 다시 불러온다.

**Tech Stack:** Tauri 2.12 (tray-icon), Rust 1.96 (rusqlite 0.40 bundled, chrono 0.4), tauri-plugin-{autostart, single-instance, opener, dialog} 2.x, React 19 + TypeScript + Vite 8, Tailwind CSS 4 (`@tailwindcss/vite`), radix-ui + class-variance-authority + clsx + tailwind-merge (shadcn/ui 방식 컴포넌트를 직접 작성), lucide-react, pretendard, Vitest 5 + Testing Library + jsdom.

**Spec:** `docs/superpowers/specs/2026-10-05-g-routine-design.md`

## Global Constraints

- 대상 OS: Windows 10/11 x64. WebView2 필요(Windows 11 기본 탑재).
- 앱 식별자 `com.groutine.app`, 제품명 `G-routine`, 버전 `0.1.0`, Rust 크레이트 `g-routine` / lib `g_routine_lib`.
- 모든 사용자 문구는 한국어로, 부드러운 존댓말("~해요", "~하세요")을 쓴다. 오류 문구는 "무엇이 일어났는지 + 할 일"을 한 문장으로 쓴다.
- 위젯 너비는 280 논리 px로 고정한다. 높이는 내용에 맞춘다(Rust에서 100~560으로 제한). 위젯은 우측 하단 기준으로 아래 모서리를 고정한 채 위로 늘어난다.
- 기본 테마는 라벤더(`#EEEDFE` `#CECBF6` `#AFA9EC` `#7F77DD` `#534AB7` `#3C3489`). 테마는 `lavender`, `mint`, `peach`, `sky`, `lemon` 5종이다.
- 칩 색 의미: 라벤더 = 매일, 민트 = 요일 지정, 피치 = 오늘만, 레몬 = 마감 시각.
- 업무일: `business_day(now) = date(now − day_start_hour)`, 기본 4시.
- 요일 비트: 월=1, 화=2, 수=4, 목=8, 금=16, 토=32, 일=64.
- 날짜 문자열은 `YYYY-MM-DD`, 시각 문자열은 로컬 `YYYY-MM-DDTHH:MM:SS`.
- 과거 업무일의 `day_items`는 절대 수정하지 않는다. 동기화(`sync_day`)는 항상 "오늘 업무일"에만 적용한다.
- "오늘만" 할 일은 이월하지 않는다. 그날이 지나면 `completed_at = NULL`로 남아 기록에서 "미완료"로 보인다.
- 주말 숨김(`hide_weekends`)은 `daily`와 `weekdays` 루틴만 숨긴다. 그 주말 날짜로 직접 지정한 `once` 항목은 보인다. 스펙의 "daily 포함"을 구체화한 것이며, 주말에 직접 적은 할 일이 사라지지 않게 하기 위함이다.
- 바로가기 허용 형식: `http://`, `https://`, 드라이브 경로(`C:\…`, `D:/…`), UNC(`\\server\…`)만 허용한다. 열기는 Rust에서 루틴 ID로만 한다.
- 데이터 파일명은 `g-routine.db`, 자동 백업은 `<data>/backups/g-routine-YYYY-MM-DD.db`(최근 7개), 위치 기록은 `%AppData%\com.groutine.app\location.json`, 포터블 모드는 exe 옆 `data\` 폴더가 있을 때다.
- 디버그 빌드에서는 자동 시작(레지스트리) 등록을 하지 않는다.
- 한국어 IME: Enter로 제출하는 입력란은 `e.nativeEvent.isComposing`이 true이면 무시한다.
- 커밋 메시지 끝에는 다음 줄을 붙인다: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

## File Structure

```text
G-routine/
├─ index.html                     창 공용 진입 HTML
├─ package.json / vite.config.ts / vitest.config.ts / tsconfig.json
├─ app-icon.svg                   아이콘 원본 (tauri icon 입력)
├─ scripts/package-portable.ps1   포터블 zip 생성
├─ docs/manual-checklist.md       수동 점검표
├─ README.md                      사용 · 설치 안내
├─ src/
│  ├─ main.tsx                    창 라벨로 WidgetApp / ManagerApp 분기
│  ├─ index.css                   Tailwind + 파스텔 테마 토큰
│  ├─ lib/utils.ts                cn()
│  ├─ lib/api.ts                  타입 + invoke 래퍼 (Rust 명령과 1:1)
│  ├─ lib/hooks.ts                useData / useSettings / useAutoResize
│  ├─ lib/windowLabel.ts          현재 창 라벨
│  ├─ dev/mockBackend.ts          브라우저 미리보기용 가짜 백엔드 (DEV 전용)
│  ├─ components/ui/{button,input,switch,tabs}.tsx   shadcn 방식 컴포넌트
│  ├─ components/Chip.tsx         매일/요일/오늘만/마감 칩
│  ├─ widget/                     위젯 창
│  │  ├─ WidgetApp.tsx  Setup.tsx  TodayPanel.tsx  WidgetHeader.tsx
│  │  ├─ TaskRow.tsx  DoneRow.tsx  UndoToast.tsx  Notice.tsx  QuickAdd.tsx
│  │  └─ today.ts (+ today.test.ts, TodayPanel.test.tsx)
│  ├─ manager/                    관리 창
│  │  ├─ ManagerApp.tsx  RoutinesTab.tsx  RoutineForm.tsx  HistoryTab.tsx  SettingsTab.tsx
│  │  └─ routines.ts  calendar.ts (+ 각 .test.ts, RoutineForm.test.tsx)
│  └─ test/setup.ts
└─ src-tauri/
   ├─ Cargo.toml  tauri.conf.json  capabilities/default.json  build.rs
   └─ src/
      ├─ main.rs  lib.rs          진입점, 플러그인 · 명령 등록
      ├─ error.rs  model.rs       오류 타입, 직렬화 모델
      ├─ domain/{mod,day,rules}.rs  업무일 · 반복 규칙 (순수 함수)
      ├─ db/{mod,routines,day_items,settings}.rs   SQLite
      ├─ service.rs               명령 하나 = 함수 하나 (테스트 대상)
      ├─ templates.rs             기본 루틴 템플릿
      ├─ storage/{mod,location,backup,export}.rs   저장 위치 · 백업
      ├─ state.rs  startup.rs     앱 상태, 부팅 · 설정 · 복구 · 폴더 변경
      ├─ shell/{mod,position,window,tray}.rs       창 · 트레이 · OS 연동
      ├─ commands.rs              Tauri 명령 (얇은 래퍼)
      └─ test_util.rs             테스트 도우미 (cfg(test))
```

---

### Task 1: 프로젝트 골격과 도구 설정

**Files:**
- Create: 저장소 루트의 Tauri React-TS 골격 전체, `vitest.config.ts`, `src/test/setup.ts`, `src/lib/utils.ts`, `src/lib/utils.test.ts`, `app-icon.svg`
- Modify: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `.gitignore`

**Interfaces:**
- Produces: `cn(...inputs)` in `src/lib/utils.ts`; 경로 별칭 `@/` → `src/`; `npm test`, `npm run build`, `cargo test` 실행 가능 상태

- [ ] **Step 1: 골격 생성 후 루트로 옮기기**

저장소 루트(`C:\Users\Hansol\Documents\G-routine`, 이미 `docs/`와 `.git`이 있음)에서 실행:

```bash
npm create tauri-app@latest g-routine -- --template react-ts --manager npm --identifier com.groutine.app --yes
cp -r g-routine/. .
rm -rf g-routine
rm -f src/App.tsx src/App.css src/assets/react.svg public/vite.svg public/tauri.svg
```

`src-tauri/src/main.rs`가 `g_routine_lib::run()`을 호출하는지 확인한다. `src-tauri/Cargo.toml`의 `[lib] name`이 `g_routine_lib`인지도 확인한다.

- [ ] **Step 2: 의존성 설치**

```bash
npm install
npm install radix-ui class-variance-authority clsx tailwind-merge lucide-react pretendard @tauri-apps/plugin-dialog
npm install -D tailwindcss @tailwindcss/vite @types/node vitest jsdom @testing-library/react @testing-library/user-event @testing-library/jest-dom
```

`@tauri-apps/plugin-opener`는 골격에 이미 있다. JS에서는 쓰지 않으므로 `npm uninstall @tauri-apps/plugin-opener`로 제거한다(Rust에서만 사용).

- [ ] **Step 3: `package.json` scripts 교체**

```json
"scripts": {
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "test": "vitest run",
  "tauri": "tauri"
}
```

`"name"`을 `"g-routine"`으로 둔다.

- [ ] **Step 4: `vite.config.ts` 작성**

```ts
import path from "node:path";
import process from "node:process";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "./src") } },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
```

- [ ] **Step 5: `vitest.config.ts`, `src/test/setup.ts` 작성**

```ts
// vitest.config.ts
import path from "node:path";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "./src") } },
  test: { environment: "jsdom", globals: true, setupFiles: ["./src/test/setup.ts"] },
});
```

```ts
// src/test/setup.ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 6: `tsconfig.json` 수정**

`compilerOptions`에 다음을 추가한다(나머지는 골격 그대로).

```json
"baseUrl": ".",
"paths": { "@/*": ["./src/*"] },
"types": ["vitest/globals"]
```

- [ ] **Step 7: `src/lib/utils.ts`와 실패하는 테스트 작성**

```ts
// src/lib/utils.test.ts
import { cn } from "@/lib/utils";

test("cn merges tailwind classes and drops falsy values", () => {
  expect(cn("px-2", false && "hidden", "px-4")).toBe("px-4");
});
```

Run: `npx vitest run src/lib/utils.test.ts` → Expected: FAIL (모듈 없음)

```ts
// src/lib/utils.ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

Run: `npm test` → Expected: PASS (1 test)

- [ ] **Step 8: 임시 진입점과 `index.html`**

```html
<!-- index.html -->
<!doctype html>
<html lang="ko">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>G-routine</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

```tsx
// src/main.tsx (Task 7에서 교체)
import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <div className="p-4 text-sm">G-routine</div>
  </React.StrictMode>,
);
```

```css
/* src/index.css (Task 7에서 교체) */
@import "tailwindcss";
```

Run: `npm run build` → Expected: 성공, `dist/` 생성

- [ ] **Step 9: Rust 의존성과 릴리스 프로필**

`src-tauri/Cargo.toml`의 `[package]`에서 `description = "교사를 위한 매일 루틴 위젯"`, `authors = ["Hansol"]`로 바꾼다. `[dependencies]`와 그 아래를 다음으로 교체한다.

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-autostart = "2"
tauri-plugin-single-instance = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.40", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3"

[profile.release]
codegen-units = 1
lto = true
opt-level = "s"
panic = "abort"
strip = true
```

Run: `cd src-tauri && cargo build` → Expected: 성공 (첫 빌드는 2분 안팎)

- [ ] **Step 10: 앱 아이콘**

```svg
<!-- app-icon.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <rect x="64" y="64" width="896" height="896" rx="220" fill="#AFA9EC"/>
  <rect x="64" y="64" width="896" height="896" rx="220" fill="none" stroke="#7F77DD" stroke-width="24"/>
  <path d="M300 530 L450 680 L730 380" fill="none" stroke="#FFFFFF" stroke-width="96" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
```

Run: `npx tauri icon app-icon.svg` → Expected: `src-tauri/icons/*` 갱신. SVG 입력이 거부되면 이 단계를 건너뛰고 골격 기본 아이콘을 유지한 뒤 보고서에 적는다.

- [ ] **Step 11: `.gitignore` 확인 후 커밋**

루트 `.gitignore`에 `node_modules`, `dist`가 있는지 확인한다. `src-tauri/.gitignore`에는 `/target/`, `/gen/schemas`가 있어야 한다.

```bash
git add -A
git commit -m "chore: scaffold Tauri 2 + React + Tailwind + Vitest project

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Rust 기반 모듈 — 오류, 모델, 업무일, 반복 규칙

**Files:**
- Create: `src-tauri/src/error.rs`, `src-tauri/src/model.rs`, `src-tauri/src/domain/mod.rs`, `src-tauri/src/domain/day.rs`, `src-tauri/src/domain/rules.rs`, `src-tauri/src/test_util.rs`
- Modify: `src-tauri/src/lib.rs` (모듈 선언만 추가)

**Interfaces:**
- Produces:
  - `error::{AppError, AppResult<T>}`. `AppError::{Db, Io, Json, Invalid(String), NotReady}`이며 Serialize하면 한국어 문자열이 된다.
  - `model::{RepeatType, Routine, RoutineInput, DayItem, TodayView, DaySummary, Settings}` (serde camelCase)
  - `domain::day::{business_day(NaiveDateTime, u32) -> NaiveDate, weekday_bit(NaiveDate) -> u8, is_weekend(NaiveDate) -> bool, fmt_day(NaiveDate) -> String, parse_day(&str) -> Option<NaiveDate>, fmt_ts(NaiveDateTime) -> String}`
  - `domain::rules::{applies(&Routine, NaiveDate) -> bool, scheduled(&[Routine], NaiveDate, bool) -> Vec<&Routine>}`
  - `test_util::{at(&str) -> NaiveDateTime, date(&str) -> NaiveDate, input_daily(&str), input_weekdays(&str, u8), input_once(&str, &str), routine(id, &str, RepeatType, u8, Option<&str>) -> Routine}`

- [ ] **Step 1: `error.rs`**

```rust
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Db(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    NotReady,
}

impl AppError {
    pub fn invalid(msg: impl Into<String>) -> Self {
        AppError::Invalid(msg.into())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(e) => write!(f, "데이터를 저장하거나 읽지 못했어요 ({e})"),
            AppError::Io(e) => write!(f, "파일을 다루지 못했어요 ({e})"),
            AppError::Json(e) => write!(f, "백업 파일 형식이 올바르지 않아요 ({e})"),
            AppError::Invalid(m) => write!(f, "{m}"),
            AppError::NotReady => write!(f, "아직 시작 설정을 마치지 않았어요"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e)
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Invalid(format!("창을 다루지 못했어요 ({e})"))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

- [ ] **Step 2: `model.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatType {
    Daily,
    Weekdays,
    Once,
}

impl RepeatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepeatType::Daily => "daily",
            RepeatType::Weekdays => "weekdays",
            RepeatType::Once => "once",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(RepeatType::Daily),
            "weekdays" => Some(RepeatType::Weekdays),
            "once" => Some(RepeatType::Once),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Routine {
    pub id: i64,
    pub title: String,
    pub repeat_type: RepeatType,
    pub weekdays: u8,
    pub once_date: Option<String>,
    pub due_time: Option<String>,
    pub link: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineInput {
    pub title: String,
    pub repeat_type: RepeatType,
    pub weekdays: u8,
    pub once_date: Option<String>,
    pub due_time: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayItem {
    pub id: i64,
    pub day: String,
    pub routine_id: i64,
    pub title: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub repeat_type: RepeatType,
    pub due_time: Option<String>,
    pub has_link: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayView {
    pub day: String,
    pub weekend_hidden: bool,
    pub pending: Vec<DayItem>,
    pub done: Vec<DayItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub day: String,
    pub total: i64,
    pub completed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub always_on_top: bool,
    pub autostart: bool,
    pub hide_weekends: bool,
    pub day_start_hour: u32,
    pub theme: String,
}
```

- [ ] **Step 3: `domain/day.rs`의 실패하는 테스트**

`src-tauri/src/domain/mod.rs`:

```rust
pub mod day;
pub mod rules;
```

`src-tauri/src/domain/day.rs` 맨 아래에 테스트를 먼저 작성한다(구현 함수는 `todo!()` 없이 아직 없음 → 컴파일 실패가 "실패"다).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{at, date};

    #[test]
    fn business_day_uses_day_start_hour_boundary() {
        assert_eq!(business_day(at("2026-10-06 01:30"), 4), date("2026-10-05"));
        assert_eq!(business_day(at("2026-10-05 03:59"), 4), date("2026-10-04"));
        assert_eq!(business_day(at("2026-10-05 04:00"), 4), date("2026-10-05"));
        assert_eq!(business_day(at("2026-10-05 00:00"), 0), date("2026-10-05"));
        assert_eq!(business_day(at("2026-10-04 23:59"), 0), date("2026-10-04"));
    }

    #[test]
    fn weekday_bits_follow_monday_first_order() {
        assert_eq!(weekday_bit(date("2026-10-05")), 1); // 월
        assert_eq!(weekday_bit(date("2026-10-09")), 16); // 금
        assert_eq!(weekday_bit(date("2026-10-11")), 64); // 일
    }

    #[test]
    fn weekend_detection() {
        assert!(!is_weekend(date("2026-10-09")));
        assert!(is_weekend(date("2026-10-10")));
        assert!(is_weekend(date("2026-10-11")));
    }

    #[test]
    fn day_and_timestamp_formatting_round_trips() {
        assert_eq!(fmt_day(date("2026-10-05")), "2026-10-05");
        assert_eq!(parse_day("2026-10-05"), Some(date("2026-10-05")));
        assert_eq!(parse_day("2026-13-05"), None);
        assert_eq!(fmt_ts(at("2026-10-05 08:47")), "2026-10-05T08:47:00");
    }
}
```

`src-tauri/src/test_util.rs`:

```rust
use chrono::{NaiveDate, NaiveDateTime};

use crate::model::{RepeatType, Routine, RoutineInput};

pub fn at(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").expect("valid datetime")
}

pub fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("valid date")
}

pub fn input_daily(title: &str) -> RoutineInput {
    RoutineInput {
        title: title.into(),
        repeat_type: RepeatType::Daily,
        weekdays: 0,
        once_date: None,
        due_time: None,
        link: None,
    }
}

pub fn input_weekdays(title: &str, weekdays: u8) -> RoutineInput {
    RoutineInput { repeat_type: RepeatType::Weekdays, weekdays, ..input_daily(title) }
}

pub fn input_once(title: &str, day: &str) -> RoutineInput {
    RoutineInput { repeat_type: RepeatType::Once, once_date: Some(day.into()), ..input_daily(title) }
}

pub fn routine(id: i64, title: &str, repeat_type: RepeatType, weekdays: u8, once_date: Option<&str>) -> Routine {
    Routine {
        id,
        title: title.into(),
        repeat_type,
        weekdays,
        once_date: once_date.map(String::from),
        due_time: None,
        link: None,
        sort_order: id,
        created_at: "2026-10-01T09:00:00".into(),
        archived_at: None,
    }
}
```

`src-tauri/src/lib.rs` 맨 위에 모듈 선언을 추가한다(기존 `greet` 예제와 `run()`은 Task 6까지 그대로 둔다).

```rust
mod domain;
mod error;
mod model;
#[cfg(test)]
mod test_util;
```

Run: `cd src-tauri && cargo test domain::day` → Expected: FAIL (컴파일 오류: `business_day` 등 없음)

- [ ] **Step 4: `domain/day.rs` 구현** (테스트 모듈 위에 둔다)

```rust
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};

/// 하루 시작 시각(day_start_hour) 이전이면 전날을 업무일로 본다.
pub fn business_day(now: NaiveDateTime, day_start_hour: u32) -> NaiveDate {
    (now - Duration::hours(i64::from(day_start_hour))).date()
}

/// 월=1, 화=2, 수=4, 목=8, 금=16, 토=32, 일=64
pub fn weekday_bit(day: NaiveDate) -> u8 {
    1u8 << day.weekday().num_days_from_monday()
}

pub fn is_weekend(day: NaiveDate) -> bool {
    matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}

pub fn fmt_day(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

pub fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

pub fn fmt_ts(t: NaiveDateTime) -> String {
    t.format("%Y-%m-%dT%H:%M:%S").to_string()
}
```

Run: `cargo test domain::day` → Expected: PASS (4 tests). `domain::rules` 파일이 없어 컴파일이 안 되면 빈 `src-tauri/src/domain/rules.rs`를 먼저 만든다.

- [ ] **Step 5: `domain/rules.rs`의 실패하는 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RepeatType;
    use crate::test_util::{date, routine};

    #[test]
    fn daily_applies_every_day_unless_archived() {
        let mut r = routine(1, "출결 확인", RepeatType::Daily, 0, None);
        assert!(applies(&r, date("2026-10-05")));
        r.archived_at = Some("2026-10-05T10:00:00".into());
        assert!(!applies(&r, date("2026-10-05")));
    }

    #[test]
    fn weekdays_match_selected_bits() {
        let r = routine(1, "주간학습안내", RepeatType::Weekdays, 16, None);
        assert!(applies(&r, date("2026-10-09")));
        assert!(!applies(&r, date("2026-10-05")));
    }

    #[test]
    fn once_matches_only_its_date() {
        let r = routine(1, "가정통신문 회수", RepeatType::Once, 0, Some("2026-10-05"));
        assert!(applies(&r, date("2026-10-05")));
        assert!(!applies(&r, date("2026-10-06")));
    }

    #[test]
    fn scheduled_sorts_by_sort_order_and_hides_recurring_on_weekends() {
        let mut a = routine(1, "A", RepeatType::Daily, 0, None);
        a.sort_order = 5;
        let b = routine(2, "B", RepeatType::Daily, 0, None);
        let once = routine(3, "주말 할 일", RepeatType::Once, 0, Some("2026-10-10"));
        let all = vec![a, b, once];

        let weekday: Vec<&str> = scheduled(&all, date("2026-10-05"), true).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(weekday, vec!["B", "A"]);

        let saturday: Vec<&str> = scheduled(&all, date("2026-10-10"), true).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(saturday, vec!["주말 할 일"]);

        let saturday_shown: Vec<&str> = scheduled(&all, date("2026-10-10"), false).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(saturday_shown, vec!["B", "주말 할 일", "A"]);
    }
}
```

`b.sort_order`는 `routine()`이 id로 채우므로 2이고, `once`는 3이다. 따라서 마지막 단언의 순서는 B(2), 주말 할 일(3), A(5)다.

Run: `cargo test domain::rules` → Expected: FAIL (`applies` 없음)

- [ ] **Step 6: `domain/rules.rs` 구현** (테스트 모듈 위)

```rust
use chrono::NaiveDate;

use crate::domain::day::{is_weekend, parse_day, weekday_bit};
use crate::model::{RepeatType, Routine};

pub fn applies(r: &Routine, day: NaiveDate) -> bool {
    if r.archived_at.is_some() {
        return false;
    }
    match r.repeat_type {
        RepeatType::Daily => true,
        RepeatType::Weekdays => r.weekdays & weekday_bit(day) != 0,
        RepeatType::Once => r.once_date.as_deref().and_then(parse_day) == Some(day),
    }
}

/// 그날 해야 할 루틴 (정렬: sort_order, id).
/// 주말 숨김이 켜져 있으면 주말에는 반복 루틴(daily/weekdays)을 빼고 once만 남긴다.
pub fn scheduled(routines: &[Routine], day: NaiveDate, hide_weekends: bool) -> Vec<&Routine> {
    let weekend_off = hide_weekends && is_weekend(day);
    let mut list: Vec<&Routine> = routines
        .iter()
        .filter(|r| applies(r, day))
        .filter(|r| !(weekend_off && r.repeat_type != RepeatType::Once))
        .collect();
    list.sort_by_key(|r| (r.sort_order, r.id));
    list
}
```

Run: `cargo test domain` → Expected: PASS (8 tests)

- [ ] **Step 7: 커밋**

```bash
git add src-tauri/src
git commit -m "feat(domain): business day and repeat rules with tests

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: SQLite 저장소 — 스키마, 루틴, 설정

**Files:**
- Create: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/routines.rs`, `src-tauri/src/db/settings.rs`, `src-tauri/src/db/day_items.rs` (빈 파일, Task 4에서 채움)
- Modify: `src-tauri/src/lib.rs` (`mod db;` 추가)

**Interfaces:**
- Consumes: `model::*`, `error::*`, `domain::day::{parse_day, fmt_day}`
- Produces:
  - `db::{open(&Path) -> AppResult<Connection>, open_in_memory() -> AppResult<Connection>, integrity_ok(&Connection) -> AppResult<bool>, DB_FILE: &str = "g-routine.db"}`
  - `db::routines::{list_unarchived(&Connection) -> AppResult<Vec<Routine>>, list_for_manager(&Connection, today: &str) -> AppResult<Vec<Routine>>, get(&Connection, i64) -> AppResult<Routine>, insert(&Connection, &RoutineInput, now: &str) -> AppResult<i64>, update(&Connection, i64, &RoutineInput) -> AppResult<()>, archive(&Connection, i64, now: &str) -> AppResult<()>, reorder(&Connection, &[i64]) -> AppResult<()>, validate(RoutineInput) -> AppResult<RoutineInput>, is_allowed_link(&str) -> bool}`
  - `db::settings::{THEMES, load(&Connection) -> AppResult<Settings>, apply(&Connection, key: &str, value: &str) -> AppResult<()>, get(&Connection, &str) -> AppResult<Option<String>>, set(&Connection, &str, &str) -> AppResult<()>, window_pos(&Connection) -> AppResult<Option<(i32, i32)>>, set_window_pos(&Connection, i32, i32) -> AppResult<()>, clear_window_pos(&Connection) -> AppResult<()>}`
  - 창 위치는 `(x, bottom)` 물리 픽셀로 저장한다 (키 `window_x`, `window_bottom`). 높이가 바뀌어도 아래 모서리가 유지되게 하기 위함이다.
  - 기본값: `always_on_top=true, autostart=true, hide_weekends=true, day_start_hour=4, theme="lavender"`

- [ ] **Step 1: `db/mod.rs`**

```rust
pub mod day_items;
pub mod routines;
pub mod settings;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::error::AppResult;

pub const DB_FILE: &str = "g-routine.db";

const SCHEMA_V1: &str = "
CREATE TABLE routines (
  id           INTEGER PRIMARY KEY,
  title        TEXT    NOT NULL,
  repeat_type  TEXT    NOT NULL CHECK (repeat_type IN ('daily','weekdays','once')),
  weekdays     INTEGER NOT NULL DEFAULT 0,
  once_date    TEXT,
  due_time     TEXT,
  link         TEXT,
  sort_order   INTEGER NOT NULL,
  created_at   TEXT    NOT NULL,
  archived_at  TEXT
);
CREATE TABLE day_items (
  id             INTEGER PRIMARY KEY,
  day            TEXT    NOT NULL,
  routine_id     INTEGER NOT NULL REFERENCES routines(id),
  title_snapshot TEXT    NOT NULL,
  sort_order     INTEGER NOT NULL,
  completed_at   TEXT,
  UNIQUE (day, routine_id)
);
CREATE INDEX idx_day_items_day ON day_items(day);
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    prepare(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    prepare(&conn)?;
    Ok(conn)
}

fn prepare(conn: &Connection) -> AppResult<()> {
    conn.busy_timeout(Duration::from_secs(3))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    migrate(conn)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA_V1)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    Ok(())
}

pub fn integrity_ok(conn: &Connection) -> AppResult<bool> {
    let result: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    Ok(result == "ok")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_tables_once_and_sets_version() {
        let conn = open_in_memory().unwrap();
        migrate(&conn).unwrap(); // 두 번째 호출은 아무것도 하지 않아야 한다
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 1);
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('routines','day_items','settings')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 3);
        assert!(integrity_ok(&conn).unwrap());
    }
}
```

`src-tauri/src/db/day_items.rs`는 이 단계에서 빈 파일로 만든다. `lib.rs`에 `mod db;`를 추가한다.

Run: `cargo test db::tests` → Expected: PASS

- [ ] **Step 2: `db/routines.rs`의 실패하는 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::model::RepeatType;
    use crate::test_util::{input_daily, input_once, input_weekdays};

    const NOW: &str = "2026-10-05T09:00:00";

    #[test]
    fn insert_assigns_increasing_sort_order_and_get_returns_row() {
        let c = open_in_memory().unwrap();
        let a = insert(&c, &input_daily("출결 확인"), NOW).unwrap();
        let b = insert(&c, &input_daily("수업 준비"), NOW).unwrap();
        assert_eq!(get(&c, a).unwrap().sort_order, 0);
        assert_eq!(get(&c, b).unwrap().sort_order, 1);
        assert_eq!(get(&c, b).unwrap().title, "수업 준비");
    }

    #[test]
    fn update_archive_and_reorder() {
        let c = open_in_memory().unwrap();
        let a = insert(&c, &input_daily("A"), NOW).unwrap();
        let b = insert(&c, &input_daily("B"), NOW).unwrap();
        update(&c, a, &input_weekdays("A2", 16)).unwrap();
        let ra = get(&c, a).unwrap();
        assert_eq!((ra.title.as_str(), ra.repeat_type, ra.weekdays), ("A2", RepeatType::Weekdays, 16));

        reorder(&c, &[b, a]).unwrap();
        let titles: Vec<String> = list_unarchived(&c).unwrap().into_iter().map(|r| r.title).collect();
        assert_eq!(titles, vec!["B", "A2"]);

        archive(&c, b, NOW).unwrap();
        assert_eq!(list_unarchived(&c).unwrap().len(), 1);
        assert!(get(&c, b).unwrap().archived_at.is_some());
        assert!(update(&c, b, &input_daily("X")).is_err());
    }

    #[test]
    fn manager_list_hides_past_once_routines() {
        let c = open_in_memory().unwrap();
        insert(&c, &input_daily("매일"), NOW).unwrap();
        insert(&c, &input_once("어제 할 일", "2026-10-04"), NOW).unwrap();
        insert(&c, &input_once("오늘 할 일", "2026-10-05"), NOW).unwrap();
        let titles: Vec<String> = list_for_manager(&c, "2026-10-05").unwrap().into_iter().map(|r| r.title).collect();
        assert_eq!(titles, vec!["매일", "오늘 할 일"]);
    }

    #[test]
    fn validate_normalizes_and_rejects_bad_input() {
        let ok = validate(RoutineInput { title: "  출결 확인 ".into(), weekdays: 5, once_date: Some("2026-10-05".into()), ..input_daily("") }).unwrap();
        assert_eq!(ok.title, "출결 확인");
        assert_eq!((ok.weekdays, ok.once_date), (0, None));

        assert!(validate(input_daily("   ")).is_err());
        assert!(validate(input_daily(&"가".repeat(41))).is_err());
        assert!(validate(input_weekdays("요일", 0)).is_err());
        assert!(validate(RoutineInput { once_date: None, ..input_once("날짜", "2026-10-05") }).is_err());
        assert!(validate(RoutineInput { due_time: Some("25:00".into()), ..input_daily("시각") }).is_err());
        assert_eq!(validate(RoutineInput { due_time: Some("".into()), ..input_daily("시각") }).unwrap().due_time, None);
        assert!(validate(RoutineInput { link: Some("javascript:alert(1)".into()), ..input_daily("링크") }).is_err());
        assert_eq!(
            validate(RoutineInput { link: Some(" https://www.neis.go.kr ".into()), ..input_daily("링크") }).unwrap().link,
            Some("https://www.neis.go.kr".into())
        );
    }

    #[test]
    fn allowed_links() {
        assert!(is_allowed_link("https://www.neis.go.kr"));
        assert!(is_allowed_link("http://edufine.go.kr"));
        assert!(is_allowed_link(r"C:\Program Files\App\app.exe"));
        assert!(is_allowed_link("D:/업무/출석부.xlsx"));
        assert!(is_allowed_link(r"\\server\share\file.hwp"));
        assert!(!is_allowed_link("https://"));
        assert!(!is_allowed_link("file:///C:/x"));
        assert!(!is_allowed_link("javascript:alert(1)"));
        assert!(!is_allowed_link("notepad.exe"));
    }
}
```

Run: `cargo test db::routines` → Expected: FAIL (함수 없음)

- [ ] **Step 3: `db/routines.rs` 구현** (테스트 모듈 위)

```rust
use chrono::NaiveTime;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::day::{fmt_day, parse_day};
use crate::error::{AppError, AppResult};
use crate::model::{RepeatType, Routine, RoutineInput};

const COLS: &str = "id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at";
const TITLE_MAX: usize = 40;

fn from_row(r: &Row) -> rusqlite::Result<Routine> {
    let repeat: String = r.get(2)?;
    let weekdays: i64 = r.get(3)?;
    Ok(Routine {
        id: r.get(0)?,
        title: r.get(1)?,
        repeat_type: RepeatType::parse(&repeat).unwrap_or(RepeatType::Daily),
        weekdays: weekdays as u8,
        once_date: r.get(4)?,
        due_time: r.get(5)?,
        link: r.get(6)?,
        sort_order: r.get(7)?,
        created_at: r.get(8)?,
        archived_at: r.get(9)?,
    })
}

pub fn list_unarchived(c: &Connection) -> AppResult<Vec<Routine>> {
    let mut st = c.prepare(&format!(
        "SELECT {COLS} FROM routines WHERE archived_at IS NULL ORDER BY sort_order, id"
    ))?;
    let rows = st.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 루틴 관리 탭용: 삭제되지 않았고, 날짜가 지난 '오늘만' 루틴이 아닌 것.
pub fn list_for_manager(c: &Connection, today: &str) -> AppResult<Vec<Routine>> {
    let mut st = c.prepare(&format!(
        "SELECT {COLS} FROM routines
         WHERE archived_at IS NULL AND NOT (repeat_type = 'once' AND once_date < ?1)
         ORDER BY sort_order, id"
    ))?;
    let rows = st.query_map([today], from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(c: &Connection, id: i64) -> AppResult<Routine> {
    c.query_row(&format!("SELECT {COLS} FROM routines WHERE id = ?1"), [id], from_row)
        .optional()?
        .ok_or_else(|| AppError::invalid("루틴을 찾을 수 없어요"))
}

pub fn insert(c: &Connection, input: &RoutineInput, now: &str) -> AppResult<i64> {
    let next: i64 = c.query_row("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM routines", [], |r| r.get(0))?;
    c.execute(
        "INSERT INTO routines (title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.title,
            input.repeat_type.as_str(),
            i64::from(input.weekdays),
            input.once_date,
            input.due_time,
            input.link,
            next,
            now
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn update(c: &Connection, id: i64, input: &RoutineInput) -> AppResult<()> {
    let n = c.execute(
        "UPDATE routines SET title = ?2, repeat_type = ?3, weekdays = ?4, once_date = ?5, due_time = ?6, link = ?7
         WHERE id = ?1 AND archived_at IS NULL",
        params![
            id,
            input.title,
            input.repeat_type.as_str(),
            i64::from(input.weekdays),
            input.once_date,
            input.due_time,
            input.link
        ],
    )?;
    if n == 0 {
        return Err(AppError::invalid("루틴을 찾을 수 없어요"));
    }
    Ok(())
}

pub fn archive(c: &Connection, id: i64, now: &str) -> AppResult<()> {
    let n = c.execute(
        "UPDATE routines SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
        params![id, now],
    )?;
    if n == 0 {
        return Err(AppError::invalid("루틴을 찾을 수 없어요"));
    }
    Ok(())
}

pub fn reorder(c: &Connection, ids: &[i64]) -> AppResult<()> {
    let tx = c.unchecked_transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute("UPDATE routines SET sort_order = ?2 WHERE id = ?1", params![id, i as i64])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn is_allowed_link(link: &str) -> bool {
    let lower = link.to_ascii_lowercase();
    for scheme in ["https://", "http://"] {
        if lower.starts_with(scheme) {
            return link.len() > scheme.len();
        }
    }
    let b = link.as_bytes();
    let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/');
    drive || link.starts_with(r"\\")
}

/// 저장 전 입력 정리 + 검증. 반복 종류에 맞지 않는 필드는 비운다.
pub fn validate(input: RoutineInput) -> AppResult<RoutineInput> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::invalid("루틴 이름을 입력해 주세요"));
    }
    if title.chars().count() > TITLE_MAX {
        return Err(AppError::invalid("루틴 이름은 40자 이내로 입력해 주세요"));
    }
    let due_time = match input.due_time.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(t) => {
            if t.len() != 5 || NaiveTime::parse_from_str(t, "%H:%M").is_err() {
                return Err(AppError::invalid("마감 시각 형식이 올바르지 않아요 (예: 09:00)"));
            }
            Some(t.to_string())
        }
    };
    let link = match input.link.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(l) => {
            if !is_allowed_link(l) {
                return Err(AppError::invalid(
                    "바로가기는 https:// 주소나 C:\\ 같은 프로그램 경로만 넣을 수 있어요",
                ));
            }
            Some(l.to_string())
        }
    };
    let (weekdays, once_date) = match input.repeat_type {
        RepeatType::Daily => (0, None),
        RepeatType::Weekdays => {
            let w = input.weekdays & 0x7F;
            if w == 0 {
                return Err(AppError::invalid("요일을 하나 이상 골라 주세요"));
            }
            (w, None)
        }
        RepeatType::Once => {
            let d = input
                .once_date
                .as_deref()
                .and_then(parse_day)
                .ok_or_else(|| AppError::invalid("날짜를 골라 주세요"))?;
            (0, Some(fmt_day(d)))
        }
    };
    Ok(RoutineInput { title, repeat_type: input.repeat_type, weekdays, once_date, due_time, link })
}
```

Run: `cargo test db::routines` → Expected: PASS (5 tests)

- [ ] **Step 4: `db/settings.rs`의 실패하는 테스트**

```rust
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
            Settings { always_on_top: true, autostart: true, hide_weekends: true, day_start_hour: 4, theme: "lavender".into() }
        );
    }

    #[test]
    fn apply_validates_and_persists() {
        let c = open_in_memory().unwrap();
        apply(&c, "hide_weekends", "false").unwrap();
        apply(&c, "day_start_hour", "5").unwrap();
        apply(&c, "theme", "mint").unwrap();
        let s = load(&c).unwrap();
        assert_eq!((s.hide_weekends, s.day_start_hour, s.theme.as_str()), (false, 5, "mint"));

        assert!(apply(&c, "hide_weekends", "yes").is_err());
        assert!(apply(&c, "day_start_hour", "24").is_err());
        assert!(apply(&c, "theme", "black").is_err());
        assert!(apply(&c, "window_x", "10").is_err());
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
```

Run: `cargo test db::settings` → Expected: FAIL

- [ ] **Step 5: `db/settings.rs` 구현** (테스트 모듈 위)

```rust
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::model::Settings;

pub const THEMES: [&str; 5] = ["lavender", "mint", "peach", "sky", "lemon"];
const BOOL_KEYS: [&str; 3] = ["always_on_top", "autostart", "hide_weekends"];

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
        day_start_hour: get(c, "day_start_hour")?
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|h| *h < 24)
            .unwrap_or(4),
        theme: get(c, "theme")?
            .filter(|t| THEMES.contains(&t.as_str()))
            .unwrap_or_else(|| "lavender".into()),
    })
}

/// 사용자가 바꿀 수 있는 설정만 검증 후 저장한다.
pub fn apply(c: &Connection, key: &str, value: &str) -> AppResult<()> {
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

pub fn window_pos(c: &Connection) -> AppResult<Option<(i32, i32)>> {
    let x = get(c, "window_x")?.and_then(|v| v.parse().ok());
    let y = get(c, "window_bottom")?.and_then(|v| v.parse().ok());
    Ok(x.zip(y))
}

pub fn set_window_pos(c: &Connection, x: i32, y: i32) -> AppResult<()> {
    set(c, "window_x", &x.to_string())?;
    set(c, "window_bottom", &y.to_string())
}

pub fn clear_window_pos(c: &Connection) -> AppResult<()> {
    remove(c, "window_x")?;
    remove(c, "window_bottom")
}
```

Run: `cargo test db` → Expected: PASS (9 tests)

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src
git commit -m "feat(db): SQLite schema, routines and settings repositories

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 4: 그날의 할 일 스냅샷, 서비스 계층, 기본 템플릿

**Files:**
- Modify: `src-tauri/src/db/day_items.rs`
- Create: `src-tauri/src/service.rs`, `src-tauri/src/templates.rs`
- Modify: `src-tauri/src/lib.rs` (`mod service; mod templates;`)

**Interfaces:**
- Consumes: Task 2 (`domain::*`, `model::*`), Task 3 (`db::routines`, `db::settings`, `db::open_in_memory`)
- Produces:
  - `db::day_items::{sync_day(&Connection, NaiveDate, hide_weekends: bool) -> AppResult<()>, items_for_day(&Connection, &str) -> AppResult<Vec<DayItem>>, set_done(&Connection, i64, bool, now: &str) -> AppResult<()>, month_summary(&Connection, i32, u32) -> AppResult<Vec<DaySummary>>}`
  - `service::{today(&Connection, NaiveDateTime) -> AppResult<NaiveDate>, get_today(&Connection, NaiveDateTime) -> AppResult<TodayView>, set_done(&Connection, i64, bool, NaiveDateTime) -> AppResult<()>, quick_add(&Connection, &str, NaiveDateTime) -> AppResult<()>, list_routines(&Connection, NaiveDateTime) -> AppResult<Vec<Routine>>, create_routine(&Connection, RoutineInput, NaiveDateTime) -> AppResult<i64>, update_routine(&Connection, i64, RoutineInput, NaiveDateTime) -> AppResult<()>, archive_routine(&Connection, i64, NaiveDateTime) -> AppResult<()>, reorder_routines(&Connection, &[i64], NaiveDateTime) -> AppResult<()>, history_month(&Connection, i32, u32) -> AppResult<Vec<DaySummary>>, history_day(&Connection, &str) -> AppResult<Vec<DayItem>>, set_setting(&Connection, &str, &str, NaiveDateTime) -> AppResult<Settings>, resync_today(&Connection, NaiveDateTime) -> AppResult<()>}`
  - `templates::{seeds(&str) -> AppResult<Vec<RoutineInput>>, apply(&Connection, &str, NaiveDateTime) -> AppResult<()>}` (템플릿 이름: `homeroom` | `subject` | `empty`)

- [ ] **Step 1: `db/day_items.rs` 구현**

이 모듈의 동작은 Step 2의 서비스 테스트로 검증한다. 스냅샷 규칙은 스펙 5.2를 따른다.

```rust
use std::collections::HashSet;

use chrono::NaiveDate;
use rusqlite::{params, Connection, Row};

use crate::domain::day::fmt_day;
use crate::domain::rules::scheduled;
use crate::db::routines;
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, RepeatType};

const ITEM_SELECT: &str = "SELECT d.id, d.day, d.routine_id, d.title_snapshot, d.sort_order, d.completed_at,
        r.repeat_type, r.due_time, r.link
   FROM day_items d JOIN routines r ON r.id = d.routine_id";

fn item_from_row(r: &Row) -> rusqlite::Result<DayItem> {
    let repeat: String = r.get(6)?;
    let link: Option<String> = r.get(8)?;
    Ok(DayItem {
        id: r.get(0)?,
        day: r.get(1)?,
        routine_id: r.get(2)?,
        title: r.get(3)?,
        sort_order: r.get(4)?,
        completed_at: r.get(5)?,
        repeat_type: RepeatType::parse(&repeat).unwrap_or(RepeatType::Daily),
        due_time: r.get(7)?,
        has_link: link.map(|l| !l.is_empty()).unwrap_or(false),
    })
}

/// 그 업무일의 할 일 스냅샷을 루틴 규칙에 맞춘다. 오늘 업무일에만 호출할 것.
/// - 새로 해당되는 루틴은 추가
/// - 더 이상 해당하지 않는 '미완료' 항목은 제거 (완료 항목은 보존)
/// - 미완료 항목의 이름과 모든 항목의 순서를 루틴과 맞춤
pub fn sync_day(c: &Connection, day: NaiveDate, hide_weekends: bool) -> AppResult<()> {
    let all = routines::list_unarchived(c)?;
    let sched = scheduled(&all, day, hide_weekends);
    let d = fmt_day(day);
    let tx = c.unchecked_transaction()?;
    for r in &sched {
        tx.execute(
            "INSERT OR IGNORE INTO day_items (day, routine_id, title_snapshot, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![d, r.id, r.title, r.sort_order],
        )?;
        tx.execute(
            "UPDATE day_items SET sort_order = ?3 WHERE day = ?1 AND routine_id = ?2",
            params![d, r.id, r.sort_order],
        )?;
        tx.execute(
            "UPDATE day_items SET title_snapshot = ?3 WHERE day = ?1 AND routine_id = ?2 AND completed_at IS NULL",
            params![d, r.id, r.title],
        )?;
    }
    let keep: HashSet<i64> = sched.iter().map(|r| r.id).collect();
    let stale: Vec<i64> = {
        let mut st = tx.prepare("SELECT id, routine_id FROM day_items WHERE day = ?1 AND completed_at IS NULL")?;
        let rows = st
            .query_map([&d], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter().filter(|(_, rid)| !keep.contains(rid)).map(|(id, _)| id).collect()
    };
    for id in stale {
        tx.execute("DELETE FROM day_items WHERE id = ?1", [id])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn items_for_day(c: &Connection, day: &str) -> AppResult<Vec<DayItem>> {
    let mut st = c.prepare(&format!("{ITEM_SELECT} WHERE d.day = ?1 ORDER BY d.sort_order, d.id"))?;
    let rows = st.query_map([day], item_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn set_done(c: &Connection, item_id: i64, done: bool, now: &str) -> AppResult<()> {
    let n = if done {
        c.execute(
            "UPDATE day_items SET completed_at = COALESCE(completed_at, ?2) WHERE id = ?1",
            params![item_id, now],
        )?
    } else {
        c.execute("UPDATE day_items SET completed_at = NULL WHERE id = ?1", [item_id])?
    };
    if n == 0 {
        return Err(AppError::invalid("항목을 찾을 수 없어요"));
    }
    Ok(())
}

pub fn month_summary(c: &Connection, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    let prefix = format!("{year:04}-{month:02}-%");
    let mut st = c.prepare(
        "SELECT day, COUNT(*), COUNT(completed_at) FROM day_items WHERE day LIKE ?1 GROUP BY day ORDER BY day",
    )?;
    let rows = st
        .query_map([prefix], |r| Ok(DaySummary { day: r.get(0)?, total: r.get(1)?, completed: r.get(2)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
```

- [ ] **Step 2: `service.rs`의 실패하는 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::test_util::{at, input_daily, input_once, input_weekdays};

    fn titles(items: &[DayItem]) -> Vec<&str> {
        items.iter().map(|i| i.title.as_str()).collect()
    }

    const MON: &str = "2026-10-05 09:00";
    const TUE: &str = "2026-10-06 09:00";
    const FRI: &str = "2026-10-09 09:00";
    const SAT: &str = "2026-10-10 09:00";

    #[test]
    fn today_lists_daily_and_matching_weekday_routines() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        create_routine(&c, input_weekdays("주간학습안내", 16), at(MON)).unwrap();

        let mon = get_today(&c, at(MON)).unwrap();
        assert_eq!(mon.day, "2026-10-05");
        assert_eq!(titles(&mon.pending), vec!["출결 확인"]);

        let fri = get_today(&c, at(FRI)).unwrap();
        assert_eq!(titles(&fri.pending), vec!["출결 확인", "주간학습안내"]);
    }

    #[test]
    fn get_today_is_idempotent() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        get_today(&c, at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert_eq!(v.pending.len(), 1);
    }

    #[test]
    fn early_morning_belongs_to_previous_business_day() {
        let c = open_in_memory().unwrap();
        let v = get_today(&c, at("2026-10-06 01:30")).unwrap();
        assert_eq!(v.day, "2026-10-05");
    }

    #[test]
    fn complete_and_undo() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();

        set_done(&c, item.id, true, at("2026-10-05 08:47")).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert!(v.pending.is_empty());
        assert_eq!(v.done[0].completed_at.as_deref(), Some("2026-10-05T08:47:00"));

        set_done(&c, item.id, false, at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert_eq!(titles(&v.pending), vec!["출결 확인"]);
        assert!(v.done.is_empty());
    }

    #[test]
    fn rename_updates_only_pending_items_of_today() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("누가기록"), at(MON)).unwrap();
        let b = create_routine(&c, input_daily("공문 확인"), at(MON)).unwrap();
        let mon = get_today(&c, at(MON)).unwrap();
        let done_id = mon.pending.iter().find(|i| i.routine_id == b).unwrap().id;
        set_done(&c, done_id, true, at(MON)).unwrap();

        update_routine(&c, a, input_daily("누가기록 작성"), at(MON)).unwrap();
        update_routine(&c, b, input_daily("공문 처리"), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert_eq!(titles(&v.pending), vec!["누가기록 작성"]);
        assert_eq!(titles(&v.done), vec!["공문 확인"]);

        // 다음 날에는 새 이름으로 생성
        let tue = get_today(&c, at(TUE)).unwrap();
        assert_eq!(titles(&tue.pending), vec!["누가기록 작성", "공문 처리"]);
        // 월요일 기록은 그대로
        assert_eq!(titles(&history_day(&c, "2026-10-05").unwrap()), vec!["누가기록 작성", "공문 확인"]);
    }

    #[test]
    fn changing_weekdays_drops_pending_but_keeps_completed_today() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("A"), at(MON)).unwrap();
        let b = create_routine(&c, input_daily("B"), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        let b_item = v.pending.iter().find(|i| i.routine_id == b).unwrap().id;
        set_done(&c, b_item, true, at(MON)).unwrap();

        update_routine(&c, a, input_weekdays("A", 2), at(MON)).unwrap(); // 화요일만
        update_routine(&c, b, input_weekdays("B", 2), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert!(v.pending.is_empty());
        assert_eq!(titles(&v.done), vec!["B"]);
    }

    #[test]
    fn once_item_expires_as_incomplete() {
        let c = open_in_memory().unwrap();
        quick_add(&c, "가정통신문 회수", at(MON)).unwrap();
        let mon = get_today(&c, at(MON)).unwrap();
        assert_eq!(titles(&mon.pending), vec!["가정통신문 회수"]);
        assert_eq!(mon.pending[0].repeat_type, RepeatType::Once);

        let tue = get_today(&c, at(TUE)).unwrap();
        assert!(tue.pending.is_empty());
        let mon_history = history_day(&c, "2026-10-05").unwrap();
        assert_eq!(mon_history.len(), 1);
        assert_eq!(mon_history[0].completed_at, None);
        assert!(list_routines(&c, at(TUE)).unwrap().is_empty());
    }

    #[test]
    fn archive_removes_pending_today_and_keeps_history() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();
        set_done(&c, item.id, true, at(MON)).unwrap();
        get_today(&c, at(TUE)).unwrap();

        archive_routine(&c, a, at(TUE)).unwrap();
        assert!(get_today(&c, at(TUE)).unwrap().pending.is_empty());
        assert_eq!(titles(&history_day(&c, "2026-10-05").unwrap()), vec!["출결 확인"]);
        assert!(list_routines(&c, at(TUE)).unwrap().is_empty());
    }

    #[test]
    fn weekend_hides_recurring_but_keeps_once() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(SAT)).unwrap();
        create_routine(&c, input_once("주말 정리", "2026-10-10"), at(SAT)).unwrap();
        let v = get_today(&c, at(SAT)).unwrap();
        assert!(v.weekend_hidden);
        assert_eq!(titles(&v.pending), vec!["주말 정리"]);

        set_setting(&c, "hide_weekends", "false", at(SAT)).unwrap();
        let v = get_today(&c, at(SAT)).unwrap();
        assert!(!v.weekend_hidden);
        assert_eq!(titles(&v.pending), vec!["출결 확인", "주말 정리"]);
    }

    #[test]
    fn reorder_changes_today_order() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("A"), at(MON)).unwrap();
        let b = create_routine(&c, input_daily("B"), at(MON)).unwrap();
        reorder_routines(&c, &[b, a], at(MON)).unwrap();
        assert_eq!(titles(&get_today(&c, at(MON)).unwrap().pending), vec!["B", "A"]);
    }

    #[test]
    fn month_summary_counts_completed() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("A"), at(MON)).unwrap();
        create_routine(&c, input_daily("B"), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        set_done(&c, v.pending[0].id, true, at(MON)).unwrap();
        get_today(&c, at(TUE)).unwrap();

        let m = history_month(&c, 2026, 10).unwrap();
        assert_eq!(
            m,
            vec![
                DaySummary { day: "2026-10-05".into(), total: 2, completed: 1 },
                DaySummary { day: "2026-10-06".into(), total: 2, completed: 0 },
            ]
        );
        assert!(history_month(&c, 2026, 13).is_err());
        assert!(history_day(&c, "2026/10/05").is_err());
    }

    #[test]
    fn invalid_input_is_rejected_without_side_effects() {
        let c = open_in_memory().unwrap();
        assert!(create_routine(&c, input_daily(" "), at(MON)).is_err());
        assert!(quick_add(&c, "  ", at(MON)).is_err());
        assert!(list_routines(&c, at(MON)).unwrap().is_empty());
    }
}
```

`lib.rs`에 `mod service;`를 추가한다.

Run: `cargo test service` → Expected: FAIL (함수 없음)

- [ ] **Step 3: `service.rs` 구현** (테스트 모듈 위)

```rust
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::Connection;

use crate::domain::day::{business_day, fmt_day, fmt_ts, is_weekend, parse_day};
use crate::db::{day_items, routines, settings};
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, RepeatType, Routine, RoutineInput, Settings, TodayView};

pub fn today(c: &Connection, now: NaiveDateTime) -> AppResult<NaiveDate> {
    let s = settings::load(c)?;
    Ok(business_day(now, s.day_start_hour))
}

pub fn resync_today(c: &Connection, now: NaiveDateTime) -> AppResult<()> {
    let s = settings::load(c)?;
    day_items::sync_day(c, business_day(now, s.day_start_hour), s.hide_weekends)
}

pub fn get_today(c: &Connection, now: NaiveDateTime) -> AppResult<TodayView> {
    let s = settings::load(c)?;
    let day = business_day(now, s.day_start_hour);
    day_items::sync_day(c, day, s.hide_weekends)?;
    let (done, pending): (Vec<DayItem>, Vec<DayItem>) = day_items::items_for_day(c, &fmt_day(day))?
        .into_iter()
        .partition(|i| i.completed_at.is_some());
    Ok(TodayView { day: fmt_day(day), weekend_hidden: s.hide_weekends && is_weekend(day), pending, done })
}

pub fn set_done(c: &Connection, item_id: i64, done: bool, now: NaiveDateTime) -> AppResult<()> {
    day_items::set_done(c, item_id, done, &fmt_ts(now))
}

pub fn quick_add(c: &Connection, title: &str, now: NaiveDateTime) -> AppResult<()> {
    let day = today(c, now)?;
    let input = RoutineInput {
        title: title.to_string(),
        repeat_type: RepeatType::Once,
        weekdays: 0,
        once_date: Some(fmt_day(day)),
        due_time: None,
        link: None,
    };
    create_routine(c, input, now).map(|_| ())
}

pub fn list_routines(c: &Connection, now: NaiveDateTime) -> AppResult<Vec<Routine>> {
    routines::list_for_manager(c, &fmt_day(today(c, now)?))
}

pub fn create_routine(c: &Connection, input: RoutineInput, now: NaiveDateTime) -> AppResult<i64> {
    let input = routines::validate(input)?;
    let id = routines::insert(c, &input, &fmt_ts(now))?;
    resync_today(c, now)?;
    Ok(id)
}

pub fn update_routine(c: &Connection, id: i64, input: RoutineInput, now: NaiveDateTime) -> AppResult<()> {
    let input = routines::validate(input)?;
    routines::update(c, id, &input)?;
    resync_today(c, now)
}

pub fn archive_routine(c: &Connection, id: i64, now: NaiveDateTime) -> AppResult<()> {
    routines::archive(c, id, &fmt_ts(now))?;
    resync_today(c, now)
}

pub fn reorder_routines(c: &Connection, ids: &[i64], now: NaiveDateTime) -> AppResult<()> {
    routines::reorder(c, ids)?;
    resync_today(c, now)
}

pub fn history_month(c: &Connection, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    if !(1..=12).contains(&month) {
        return Err(AppError::invalid("월 값이 올바르지 않아요"));
    }
    day_items::month_summary(c, year, month)
}

pub fn history_day(c: &Connection, day: &str) -> AppResult<Vec<DayItem>> {
    let d = parse_day(day).ok_or_else(|| AppError::invalid("날짜 형식이 올바르지 않아요"))?;
    day_items::items_for_day(c, &fmt_day(d))
}

pub fn set_setting(c: &Connection, key: &str, value: &str, now: NaiveDateTime) -> AppResult<Settings> {
    settings::apply(c, key, value)?;
    if key == "hide_weekends" || key == "day_start_hour" {
        resync_today(c, now)?;
    }
    settings::load(c)
}
```

Run: `cargo test service` → Expected: PASS (12 tests)

- [ ] **Step 4: `templates.rs` — 테스트와 구현**

```rust
use chrono::NaiveDateTime;
use rusqlite::Connection;

use crate::domain::day::fmt_ts;
use crate::db::routines;
use crate::error::{AppError, AppResult};
use crate::model::{RepeatType, RoutineInput};

fn item(title: &str, repeat_type: RepeatType, weekdays: u8, link: Option<&str>) -> RoutineInput {
    RoutineInput {
        title: title.into(),
        repeat_type,
        weekdays,
        once_date: None,
        due_time: None,
        link: link.map(String::from),
    }
}

pub fn seeds(template: &str) -> AppResult<Vec<RoutineInput>> {
    use RepeatType::{Daily, Weekdays};
    match template {
        "homeroom" => Ok(vec![
            item("출결 확인", Daily, 0, Some("https://www.neis.go.kr")),
            item("수업 준비", Daily, 0, None),
            item("누가기록 작성", Daily, 0, None),
            item("공문 확인", Daily, 0, None),
            item("알림장 작성", Daily, 0, None),
            item("주간학습안내 배부", Weekdays, 16, None),
        ]),
        "subject" => Ok(vec![
            item("수업 준비", Daily, 0, None),
            item("공문 확인", Daily, 0, None),
            item("수업 기록 · 진도 체크", Daily, 0, None),
            item("수행평가 기록", Daily, 0, None),
        ]),
        "empty" => Ok(vec![]),
        _ => Err(AppError::invalid("알 수 없는 템플릿이에요")),
    }
}

pub fn apply(c: &Connection, template: &str, now: NaiveDateTime) -> AppResult<()> {
    for input in seeds(template)? {
        routines::insert(c, &routines::validate(input)?, &fmt_ts(now))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::test_util::at;

    #[test]
    fn templates_seed_expected_counts() {
        assert_eq!(seeds("homeroom").unwrap().len(), 6);
        assert_eq!(seeds("subject").unwrap().len(), 4);
        assert!(seeds("empty").unwrap().is_empty());
        assert!(seeds("unknown").is_err());
    }

    #[test]
    fn apply_inserts_valid_routines() {
        let c = open_in_memory().unwrap();
        apply(&c, "homeroom", at("2026-10-05 08:00")).unwrap();
        let list = routines::list_unarchived(&c).unwrap();
        assert_eq!(list[0].title, "출결 확인");
        assert_eq!(list[0].link.as_deref(), Some("https://www.neis.go.kr"));
        assert_eq!(list[5].weekdays, 16);
    }
}
```

`lib.rs`에 `mod templates;`를 추가한다.

Run: `cargo test` → Expected: 지금까지의 테스트 전부 PASS

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src
git commit -m "feat(service): day snapshots, routine operations, history and templates

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: 저장 위치 해석, 자동 백업, JSON 백업 내보내기 · 가져오기

**Files:**
- Create: `src-tauri/src/storage/mod.rs`, `src-tauri/src/storage/location.rs`, `src-tauri/src/storage/backup.rs`, `src-tauri/src/storage/export.rs`
- Modify: `src-tauri/src/lib.rs` (`mod storage;`)

**Interfaces:**
- Consumes: `db::{open, open_in_memory, DB_FILE}`, `model::Routine`, `error::*`, `service` (테스트용)
- Produces:
  - `storage::location::{Resolution { dir: Option<PathBuf>, portable: bool, previous: Option<PathBuf> }, resolve(exe_dir: &Path, location_file: &Path, candidates: &[PathBuf]) -> Resolution, read_location(&Path) -> Option<PathBuf>, write_location(&Path, &Path) -> AppResult<()>, drive_candidates() -> Vec<PathBuf>, suggested_dir() -> PathBuf}`
  - `storage::backup::{daily_backup(&Connection, data_dir: &Path, day: &str) -> AppResult<Option<PathBuf>>, latest_backup(&Path) -> Option<PathBuf>, restore_latest(&Path) -> AppResult<()>, KEEP: usize = 7}`
  - `storage::export::{BackupFile, DayItemRow, export(&Connection, now: &str) -> AppResult<BackupFile>, import(&Connection, &BackupFile) -> AppResult<()>, write_file(&Path, &BackupFile) -> AppResult<()>, read_file(&Path) -> AppResult<BackupFile>}`

- [ ] **Step 1: `storage/location.rs` — 실패하는 테스트**

`src-tauri/src/storage/mod.rs`:

```rust
pub mod backup;
pub mod export;
pub mod location;
```

`location.rs` 하단:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DB_FILE;
    use std::fs;

    fn touch_db(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(DB_FILE), b"x").unwrap();
    }

    #[test]
    fn portable_data_folder_wins() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("app");
        fs::create_dir_all(exe.join("data")).unwrap();
        let r = resolve(&exe, &t.path().join("cfg/location.json"), &[]);
        assert_eq!(r.dir, Some(exe.join("data")));
        assert!(r.portable);
    }

    #[test]
    fn recorded_location_with_db_is_used() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("D/G-routine/data");
        touch_db(&data);
        let loc = t.path().join("cfg/location.json");
        write_location(&loc, &data).unwrap();
        let r = resolve(&t.path().join("app"), &loc, &[]);
        assert_eq!(r.dir, Some(data));
        assert!(!r.portable);
    }

    #[test]
    fn falls_back_to_candidates_and_rewrites_location() {
        let t = tempfile::tempdir().unwrap();
        let empty = t.path().join("E/G-routine/data");
        let found = t.path().join("D/G-routine/data");
        touch_db(&found);
        let loc = t.path().join("cfg/location.json"); // 복원 프로그램이 지워서 없음
        let r = resolve(&t.path().join("app"), &loc, &[empty, found.clone()]);
        assert_eq!(r.dir, Some(found.clone()));
        assert_eq!(read_location(&loc), Some(found));
    }

    #[test]
    fn needs_setup_reports_previous_missing_dir() {
        let t = tempfile::tempdir().unwrap();
        let loc = t.path().join("cfg/location.json");
        let gone = t.path().join("Z/G-routine/data");
        write_location(&loc, &gone).unwrap();
        let r = resolve(&t.path().join("app"), &loc, &[]);
        assert_eq!(r.dir, None);
        assert_eq!(r.previous, Some(gone));
    }
}
```

Run: `cargo test storage::location` → Expected: FAIL

- [ ] **Step 2: `storage/location.rs` 구현** (테스트 위)

```rust
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db::DB_FILE;
use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    /// 사용할 데이터 폴더. Some이어도 DB 파일이 아직 없을 수 있다 (포터블 첫 실행).
    pub dir: Option<PathBuf>,
    pub portable: bool,
    /// location.json에 적혀 있었지만 찾지 못한 폴더
    pub previous: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocationFile {
    data_dir: PathBuf,
}

pub fn read_location(file: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(file).ok()?;
    serde_json::from_str::<LocationFile>(&text).ok().map(|l| l.data_dir)
}

pub fn write_location(file: &Path, dir: &Path) -> AppResult<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&LocationFile { data_dir: dir.to_path_buf() })?;
    fs::write(file, text)?;
    Ok(())
}

/// 저장 위치 결정 순서: ① exe 옆 data 폴더(포터블) ② location.json ③ 후보 드라이브 탐색
pub fn resolve(exe_dir: &Path, location_file: &Path, candidates: &[PathBuf]) -> Resolution {
    let portable = exe_dir.join("data");
    if portable.is_dir() {
        return Resolution { dir: Some(portable), portable: true, previous: None };
    }
    let previous = read_location(location_file);
    if let Some(p) = &previous {
        if p.join(DB_FILE).is_file() {
            return Resolution { dir: Some(p.clone()), portable: false, previous: None };
        }
    }
    for c in candidates {
        if c.join(DB_FILE).is_file() {
            let _ = write_location(location_file, c);
            return Resolution { dir: Some(c.clone()), portable: false, previous: None };
        }
    }
    Resolution { dir: None, portable: false, previous }
}

/// D:~Z: 중 존재하는 드라이브의 \G-routine\data (D 우선)
pub fn drive_candidates() -> Vec<PathBuf> {
    ('D'..='Z')
        .map(|l| PathBuf::from(format!("{l}:\\")))
        .filter(|root| root.exists())
        .map(|root| root.join("G-routine").join("data"))
        .collect()
}

pub fn suggested_dir() -> PathBuf {
    let d = PathBuf::from("D:\\");
    if d.exists() {
        return d.join("G-routine").join("data");
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\"));
    home.join("Documents").join("G-routine").join("data")
}
```

Run: `cargo test storage::location` → Expected: PASS (4 tests)

- [ ] **Step 3: `storage/backup.rs` — 테스트와 구현**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db::DB_FILE;
use crate::error::{AppError, AppResult};

pub const KEEP: usize = 7;
const PREFIX: &str = "g-routine-";

fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn list_backups(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    name.starts_with(PREFIX) && name.ends_with(".db")
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// 업무일마다 한 번 DB 사본을 만든다. 이미 있으면 None.
pub fn daily_backup(c: &Connection, data_dir: &Path, day: &str) -> AppResult<Option<PathBuf>> {
    let dir = backups_dir(data_dir);
    fs::create_dir_all(&dir)?;
    let target = dir.join(format!("{PREFIX}{day}.db"));
    if target.exists() {
        return Ok(None);
    }
    c.execute("VACUUM INTO ?1", [target.to_string_lossy().to_string()])?;
    let files = list_backups(&dir);
    if files.len() > KEEP {
        for old in &files[..files.len() - KEEP] {
            let _ = fs::remove_file(old);
        }
    }
    Ok(Some(target))
}

pub fn latest_backup(data_dir: &Path) -> Option<PathBuf> {
    list_backups(&backups_dir(data_dir)).pop()
}

pub fn restore_latest(data_dir: &Path) -> AppResult<()> {
    let latest = latest_backup(data_dir).ok_or_else(|| AppError::invalid("복구할 백업 파일이 없어요"))?;
    fs::copy(latest, data_dir.join(DB_FILE))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn daily_backup_once_per_day_and_rotates() {
        let t = tempfile::tempdir().unwrap();
        let c = db::open(&t.path().join(DB_FILE)).unwrap();
        assert!(daily_backup(&c, t.path(), "2026-10-01").unwrap().is_some());
        assert!(daily_backup(&c, t.path(), "2026-10-01").unwrap().is_none());
        for d in 2..=9 {
            daily_backup(&c, t.path(), &format!("2026-10-0{d}")).unwrap();
        }
        let files = list_backups(&backups_dir(t.path()));
        assert_eq!(files.len(), KEEP);
        assert!(files[0].ends_with("g-routine-2026-10-03.db"));
        assert!(latest_backup(t.path()).unwrap().ends_with("g-routine-2026-10-09.db"));
    }

    #[test]
    fn restore_latest_replaces_db_file() {
        let t = tempfile::tempdir().unwrap();
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            daily_backup(&c, t.path(), "2026-10-05").unwrap();
        }
        fs::write(t.path().join(DB_FILE), b"broken").unwrap();
        restore_latest(t.path()).unwrap();
        let c = db::open(&t.path().join(DB_FILE)).unwrap();
        assert!(db::integrity_ok(&c).unwrap());
    }

    #[test]
    fn restore_without_backups_fails() {
        let t = tempfile::tempdir().unwrap();
        assert!(restore_latest(t.path()).is_err());
    }
}
```

`lib.rs`에 `mod storage;`를 추가한다.

Run: `cargo test storage::backup` → Expected: PASS (3 tests)

- [ ] **Step 4: `storage/export.rs` — 테스트와 구현**

```rust
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::routines;
use crate::error::{AppError, AppResult};
use crate::model::Routine;

const APP: &str = "g-routine";
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayItemRow {
    pub id: i64,
    pub day: String,
    pub routine_id: i64,
    pub title_snapshot: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub app: String,
    pub version: u32,
    pub exported_at: String,
    pub routines: Vec<Routine>,
    pub day_items: Vec<DayItemRow>,
    pub settings: Vec<(String, String)>,
}

fn all_routines(c: &Connection) -> AppResult<Vec<Routine>> {
    let mut ids = c.prepare("SELECT id FROM routines ORDER BY id")?;
    let ids: Vec<i64> = ids.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    ids.into_iter().map(|id| routines::get(c, id)).collect()
}

pub fn export(c: &Connection, now: &str) -> AppResult<BackupFile> {
    let mut st = c.prepare(
        "SELECT id, day, routine_id, title_snapshot, sort_order, completed_at FROM day_items ORDER BY id",
    )?;
    let day_items = st
        .query_map([], |r| {
            Ok(DayItemRow {
                id: r.get(0)?,
                day: r.get(1)?,
                routine_id: r.get(2)?,
                title_snapshot: r.get(3)?,
                sort_order: r.get(4)?,
                completed_at: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut st = c.prepare("SELECT key, value FROM settings WHERE key NOT LIKE 'window_%' ORDER BY key")?;
    let settings = st
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BackupFile {
        app: APP.into(),
        version: VERSION,
        exported_at: now.into(),
        routines: all_routines(c)?,
        day_items,
        settings,
    })
}

/// 현재 데이터를 백업 내용으로 통째로 바꾼다 (창 위치 설정은 유지).
pub fn import(c: &Connection, b: &BackupFile) -> AppResult<()> {
    validate(b)?;
    let tx = c.unchecked_transaction()?;
    tx.execute("DELETE FROM day_items", [])?;
    tx.execute("DELETE FROM routines", [])?;
    tx.execute("DELETE FROM settings WHERE key NOT LIKE 'window_%'", [])?;
    for r in &b.routines {
        tx.execute(
            "INSERT INTO routines (id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                r.id,
                r.title,
                r.repeat_type.as_str(),
                i64::from(r.weekdays),
                r.once_date,
                r.due_time,
                r.link,
                r.sort_order,
                r.created_at,
                r.archived_at
            ],
        )?;
    }
    for d in &b.day_items {
        tx.execute(
            "INSERT INTO day_items (id, day, routine_id, title_snapshot, sort_order, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![d.id, d.day, d.routine_id, d.title_snapshot, d.sort_order, d.completed_at],
        )?;
    }
    for (k, v) in &b.settings {
        tx.execute("INSERT INTO settings (key, value) VALUES (?1, ?2)", params![k, v])?;
    }
    tx.commit()?;
    Ok(())
}

fn validate(b: &BackupFile) -> AppResult<()> {
    if b.app != APP || b.version != VERSION {
        return Err(AppError::invalid("G-routine 백업 파일이 아니에요"));
    }
    Ok(())
}

pub fn write_file(path: &Path, b: &BackupFile) -> AppResult<()> {
    fs::write(path, serde_json::to_string_pretty(b)?)?;
    Ok(())
}

pub fn read_file(path: &Path) -> AppResult<BackupFile> {
    let b: BackupFile = serde_json::from_str(&fs::read_to_string(path)?)?;
    validate(&b)?;
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_in_memory, settings};
    use crate::service;
    use crate::test_util::{at, input_daily};

    #[test]
    fn export_then_import_round_trips_into_fresh_db() {
        let src = open_in_memory().unwrap();
        service::create_routine(&src, input_daily("출결 확인"), at("2026-10-05 09:00")).unwrap();
        let v = service::get_today(&src, at("2026-10-05 09:00")).unwrap();
        service::set_done(&src, v.pending[0].id, true, at("2026-10-05 09:10")).unwrap();
        settings::apply(&src, "theme", "mint").unwrap();
        settings::set_window_pos(&src, 10, 10).unwrap();
        let backup = export(&src, "2026-10-05T10:00:00").unwrap();
        assert_eq!(backup.settings, vec![("theme".to_string(), "mint".to_string())]);

        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("b.json");
        write_file(&path, &backup).unwrap();

        let dst = open_in_memory().unwrap();
        service::create_routine(&dst, input_daily("지워질 루틴"), at("2026-10-05 09:00")).unwrap();
        import(&dst, &read_file(&path).unwrap()).unwrap();
        let history = service::history_day(&dst, "2026-10-05").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].title, "출결 확인");
        assert_eq!(history[0].completed_at.as_deref(), Some("2026-10-05T09:10:00"));
        assert_eq!(settings::load(&dst).unwrap().theme, "mint");
    }

    #[test]
    fn rejects_foreign_json() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("x.json");
        fs::write(&path, r#"{"app":"other","version":1,"exportedAt":"","routines":[],"dayItems":[],"settings":[]}"#).unwrap();
        assert!(read_file(&path).is_err());
        fs::write(&path, "not json").unwrap();
        assert!(read_file(&path).is_err());
    }
}
```

Run: `cargo test storage` → Expected: PASS (9 tests)

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src
git commit -m "feat(storage): data folder resolution, daily backups, JSON export/import

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 6: 앱 셸 — 상태, 부팅 · 설정 · 복구, 창 · 트레이, 명령, Tauri 설정

**Files:**
- Create: `src-tauri/src/state.rs`, `src-tauri/src/startup.rs`, `src-tauri/src/shell/mod.rs`, `src-tauri/src/shell/position.rs`, `src-tauri/src/shell/window.rs`, `src-tauri/src/shell/tray.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (전체 교체), `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: Task 2~5의 모든 공개 함수
- Produces (프런트엔드가 의존하는 계약):
  - 이벤트 `data-changed` (payload 없음). 데이터를 바꾸는 모든 명령이 끝날 때와 업무일이 바뀔 때 발생한다.
  - Tauri 명령(인자 이름은 JS에서 camelCase로 넘긴다):
    `get_status() -> AppStatus`, `setup(dir, template)`, `restore_backup()`, `get_today() -> TodayView`, `set_done(itemId, done)`, `quick_add(title)`, `list_routines() -> Routine[]`, `create_routine(input) -> number`, `update_routine(id, input)`, `archive_routine(id)`, `reorder_routines(ids)`, `history_month(year, month) -> DaySummary[]`, `history_day(day) -> DayItem[]`, `get_settings() -> Settings`, `set_setting(key, value) -> Settings`, `open_link(routineId)`, `export_backup(path)`, `import_backup(path)`, `change_data_dir(dir)`, `open_manager()`, `resize_widget(height)`, `hide_widget()`
  - `AppStatus { ready, corrupt, portable, previousDir, suggestedDir, dataDir }` (camelCase JSON)
  - 오류는 한국어 문자열로 reject된다.
  - 창 라벨: `widget`(설정 파일에 정의), `manager`(Rust에서 필요할 때 생성)

- [ ] **Step 1: `state.rs`**

```rust
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;
use std::time::Instant;

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub ready: bool,
    pub corrupt: bool,
    pub portable: bool,
    pub previous_dir: Option<String>,
    pub suggested_dir: String,
    pub data_dir: Option<String>,
}

pub struct AppState {
    conn: Mutex<Option<Connection>>,
    status: Mutex<AppStatus>,
    pub location_file: PathBuf,
    pub move_seq: AtomicU64,
    pub programmatic_move: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn new(location_file: PathBuf) -> Self {
        AppState {
            conn: Mutex::new(None),
            status: Mutex::new(AppStatus::default()),
            location_file,
            move_seq: AtomicU64::new(0),
            programmatic_move: Mutex::new(None),
        }
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self.conn.lock().map_err(|_| AppError::invalid("내부 상태를 읽지 못했어요"))?;
        match guard.as_ref() {
            Some(c) => f(c),
            None => Err(AppError::NotReady),
        }
    }

    pub fn replace_conn(&self, conn: Option<Connection>) {
        if let Ok(mut g) = self.conn.lock() {
            *g = conn;
        }
    }

    pub fn status(&self) -> AppStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn update_status(&self, f: impl FnOnce(&mut AppStatus)) {
        if let Ok(mut g) = self.status.lock() {
            f(&mut g);
        }
    }

    pub fn data_dir(&self) -> Option<PathBuf> {
        self.status().data_dir.map(PathBuf::from)
    }
}
```

- [ ] **Step 2: `startup.rs`의 실패하는 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::routines;
    use crate::service;
    use crate::test_util::at;

    const NOW: &str = "2026-10-05 09:00";

    fn state_in(root: &Path) -> AppState {
        AppState::new(root.join("cfg").join("location.json"))
    }

    fn routine_count(s: &AppState) -> usize {
        s.with_conn(|c| Ok(routines::list_unarchived(c)?.len())).unwrap()
    }

    #[test]
    fn first_boot_needs_setup_then_setup_seeds_template() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("app");
        let data = t.path().join("D").join("G-routine").join("data");
        let state = state_in(t.path());
        boot(&state, &exe, &[], at(NOW));
        assert!(!state.status().ready);

        setup(&state, &data, "homeroom", at(NOW)).unwrap();
        let st = state.status();
        assert!(st.ready);
        assert_eq!(st.data_dir, Some(data.display().to_string()));
        let v = state.with_conn(|c| service::get_today(c, at(NOW))).unwrap();
        assert_eq!(v.pending.len(), 5); // 금요일 전용 1개 제외
        assert!(data.join("backups").join("g-routine-2026-10-05.db").is_file());

        drop(state);
        let again = state_in(t.path());
        boot(&again, &exe, &[], at(NOW));
        assert!(again.status().ready);
    }

    #[test]
    fn boot_reconnects_from_candidates_when_location_file_is_wiped() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("D").join("G-routine").join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "empty", at(NOW)).unwrap();
        }
        fs::remove_dir_all(t.path().join("cfg")).unwrap();
        let state = state_in(t.path());
        boot(&state, &t.path().join("app"), &[data.clone()], at(NOW));
        assert!(state.status().ready);
        assert_eq!(location::read_location(&state.location_file), Some(data));
    }

    #[test]
    fn setup_on_existing_db_keeps_data_and_ignores_template() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "subject", at(NOW)).unwrap();
        }
        let s = state_in(t.path());
        setup(&s, &data, "homeroom", at(NOW)).unwrap();
        assert_eq!(routine_count(&s), 4);
    }

    #[test]
    fn setup_rejects_unknown_template_before_touching_disk() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        assert!(setup(&s, &t.path().join("data"), "nope", at(NOW)).is_err());
        assert!(!t.path().join("data").exists());
    }

    #[test]
    fn portable_mode_uses_exe_data_folder() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("app");
        fs::create_dir_all(exe.join("data")).unwrap();
        let s = state_in(t.path());
        boot(&s, &exe, &[], at(NOW));
        let st = s.status();
        assert!(st.portable && !st.ready);

        setup(&s, Path::new("ignored"), "empty", at(NOW)).unwrap();
        assert!(exe.join("data").join(DB_FILE).is_file());
        assert!(!s.location_file.exists());
        assert!(change_dir(&s, &t.path().join("other")).is_err());
    }

    #[test]
    fn corrupt_db_is_reported_and_restorable_from_backup() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "homeroom", at(NOW)).unwrap();
        }
        fs::write(data.join(DB_FILE), b"this is not a database file at all").unwrap();
        let s = state_in(t.path());
        boot(&s, &t.path().join("app"), &[], at(NOW));
        let st = s.status();
        assert!(st.corrupt && !st.ready);
        assert_eq!(st.previous_dir, Some(data.display().to_string()));

        restore(&s, at(NOW)).unwrap();
        assert!(s.status().ready);
        assert_eq!(routine_count(&s), 6);
    }

    #[test]
    fn change_dir_copies_database_and_updates_location() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        setup(&s, &t.path().join("a"), "subject", at(NOW)).unwrap();
        let b = t.path().join("b");
        change_dir(&s, &b).unwrap();
        assert!(b.join(DB_FILE).is_file());
        assert_eq!(s.status().data_dir, Some(b.display().to_string()));
        assert_eq!(location::read_location(&s.location_file), Some(b));
        assert_eq!(routine_count(&s), 4);
    }
}
```

`lib.rs`에 `mod state; mod startup;`을 추가한다(Step 8에서 lib.rs 전체를 교체한다).

Run: `cargo test startup` → Expected: FAIL (함수 없음)

- [ ] **Step 3: `startup.rs` 구현** (테스트 위)

```rust
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use rusqlite::Connection;

use crate::db::{self, settings, DB_FILE};
use crate::domain::day::{business_day, fmt_day};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, AppStatus};
use crate::storage::{backup, location};
use crate::templates;

pub fn open_checked(dir: &Path) -> AppResult<Connection> {
    let conn = db::open(&dir.join(DB_FILE))?;
    if !db::integrity_ok(&conn)? {
        return Err(AppError::invalid("데이터 파일이 손상되었어요"));
    }
    Ok(conn)
}

pub fn run_daily_backup(c: &Connection, dir: &Path, now: NaiveDateTime) -> AppResult<()> {
    let s = settings::load(c)?;
    backup::daily_backup(c, dir, &fmt_day(business_day(now, s.day_start_hour)))?;
    Ok(())
}

fn mark_ready(state: &AppState, dir: &Path, conn: Connection) {
    state.replace_conn(Some(conn));
    state.update_status(|s| {
        s.ready = true;
        s.corrupt = false;
        s.previous_dir = None;
        s.data_dir = Some(dir.display().to_string());
        s.suggested_dir = dir.display().to_string();
    });
}

/// 앱 시작 시 저장 위치를 찾고, DB가 있으면 열고 그날 첫 백업을 만든다.
pub fn boot(state: &AppState, exe_dir: &Path, candidates: &[PathBuf], now: NaiveDateTime) {
    let res = location::resolve(exe_dir, &state.location_file, candidates);
    let suggested = res.dir.clone().unwrap_or_else(location::suggested_dir);
    state.update_status(|s| {
        *s = AppStatus {
            portable: res.portable,
            previous_dir: res.previous.as_ref().map(|p| p.display().to_string()),
            suggested_dir: suggested.display().to_string(),
            ..AppStatus::default()
        };
    });
    let Some(dir) = res.dir else { return };
    if !dir.join(DB_FILE).is_file() {
        return;
    }
    match open_checked(&dir) {
        Ok(conn) => {
            let _ = run_daily_backup(&conn, &dir, now);
            mark_ready(state, &dir, conn);
        }
        Err(_) => state.update_status(|s| {
            s.corrupt = true;
            s.previous_dir = Some(dir.display().to_string());
        }),
    }
}

/// 첫 실행 설정. 폴더에 DB가 이미 있으면 그대로 연결하고 템플릿은 무시한다.
pub fn setup(state: &AppState, dir: &Path, template: &str, now: NaiveDateTime) -> AppResult<()> {
    templates::seeds(template)?;
    let st = state.status();
    let dir: PathBuf = if st.portable { PathBuf::from(&st.suggested_dir) } else { dir.to_path_buf() };
    if dir.as_os_str().is_empty() {
        return Err(AppError::invalid("저장할 폴더를 골라 주세요"));
    }
    fs::create_dir_all(&dir)?;
    let conn = if dir.join(DB_FILE).is_file() {
        open_checked(&dir)?
    } else {
        let c = db::open(&dir.join(DB_FILE))?;
        templates::apply(&c, template, now)?;
        c
    };
    if !st.portable {
        location::write_location(&state.location_file, &dir)?;
    }
    let _ = run_daily_backup(&conn, &dir, now);
    mark_ready(state, &dir, conn);
    Ok(())
}

/// 손상된 DB를 가장 최근 자동 백업으로 되돌린다.
pub fn restore(state: &AppState, now: NaiveDateTime) -> AppResult<()> {
    let st = state.status();
    let dir = st
        .previous_dir
        .map(PathBuf::from)
        .ok_or_else(|| AppError::invalid("복구할 폴더를 찾지 못했어요"))?;
    backup::restore_latest(&dir)?;
    let conn = open_checked(&dir)?;
    if !st.portable {
        location::write_location(&state.location_file, &dir)?;
    }
    let _ = run_daily_backup(&conn, &dir, now);
    mark_ready(state, &dir, conn);
    Ok(())
}

/// 저장 폴더 변경. 새 폴더에 DB가 있으면 그것에 연결하고, 없으면 현재 DB를 복사한다.
pub fn change_dir(state: &AppState, new_dir: &Path) -> AppResult<()> {
    let st = state.status();
    if st.portable {
        return Err(AppError::invalid("포터블 모드에서는 저장 위치를 바꿀 수 없어요"));
    }
    if st.data_dir.as_deref().map(Path::new) == Some(new_dir) {
        return Ok(());
    }
    fs::create_dir_all(new_dir)?;
    let target = new_dir.join(DB_FILE);
    if !target.is_file() {
        state.with_conn(|c| {
            c.execute("VACUUM INTO ?1", [target.to_string_lossy().to_string()])?;
            Ok(())
        })?;
    }
    let conn = open_checked(new_dir)?;
    location::write_location(&state.location_file, new_dir)?;
    mark_ready(state, new_dir, conn);
    Ok(())
}
```

Run: `cargo test startup` → Expected: PASS (7 tests)

- [ ] **Step 4: `shell/position.rs` — 테스트와 구현**

```rust
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
```

`src-tauri/src/shell/mod.rs`는 Step 7에서 완성한다. 지금은 다음만 넣는다.

```rust
pub mod position;
```

`lib.rs`에 `mod shell;`을 추가한다.

Run: `cargo test shell::position` → Expected: PASS (3 tests)

- [ ] **Step 5: `shell/window.rs`**

```rust
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::db::settings;
use crate::service;
use crate::shell::position::{anchored_y, bottom_right, center_inside, Rect};
use crate::state::AppState;
use crate::{now, startup, DATA_CHANGED};

pub const WIDGET: &str = "widget";
pub const MANAGER: &str = "manager";
const MARGIN: f64 = 12.0;
const MIN_HEIGHT: f64 = 100.0;
const MAX_HEIGHT: f64 = 560.0;

pub fn show_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.hide();
    }
}

fn mark_programmatic_move(app: &AppHandle) {
    if let Ok(mut g) = app.state::<AppState>().programmatic_move.lock() {
        *g = Some(Instant::now());
    }
}

/// saved = (x, 아래쪽 y). 저장 위치가 화면 밖이면 주 모니터 작업 영역 우측 하단에 둔다.
pub fn place_widget(app: &AppHandle, saved: Option<(i32, i32)>) -> tauri::Result<()> {
    let Some(w) = app.get_webview_window(WIDGET) else { return Ok(()) };
    let size = w.outer_size()?;
    let (ww, wh) = (size.width as i32, size.height as i32);
    let monitors: Vec<Rect> = w
        .available_monitors()?
        .iter()
        .map(|m| Rect { x: m.position().x, y: m.position().y, w: m.size().width as i32, h: m.size().height as i32 })
        .collect();
    let pos = match saved {
        Some((x, bottom)) if center_inside(x, bottom - wh, ww, wh, &monitors) => (x, bottom - wh),
        _ => {
            let Some(m) = w.primary_monitor()?.or(w.current_monitor()?) else { return Ok(()) };
            let wa = m.work_area();
            let work = Rect { x: wa.position.x, y: wa.position.y, w: wa.size.width as i32, h: wa.size.height as i32 };
            let margin = (MARGIN * m.scale_factor()).round() as i32;
            bottom_right(work, ww, wh, margin)
        }
    };
    mark_programmatic_move(app);
    w.set_position(PhysicalPosition::new(pos.0, pos.1))
}

pub fn reset_position(app: &AppHandle) {
    let _ = app.state::<AppState>().with_conn(settings::clear_window_pos);
    let _ = place_widget(app, None);
    show_widget(app);
}

/// 내용 높이(논리 px)에 맞춰 위젯 높이를 바꾸되 아래 모서리를 고정한다.
pub fn resize_widget(app: &AppHandle, logical_height: f64) -> tauri::Result<()> {
    let Some(w) = app.get_webview_window(WIDGET) else { return Ok(()) };
    let scale = w.scale_factor()?;
    let pos = w.outer_position()?;
    let old = w.outer_size()?;
    let new_h = (logical_height.clamp(MIN_HEIGHT, MAX_HEIGHT) * scale).round() as i32;
    if new_h == old.height as i32 {
        return Ok(());
    }
    let y = anchored_y(pos.y, old.height as i32, new_h);
    mark_programmatic_move(app);
    w.set_size(PhysicalSize::new(old.width, new_h as u32))?;
    w.set_position(PhysicalPosition::new(pos.x, y))
}

pub fn open_manager(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(MANAGER) {
        w.show()?;
        w.unminimize()?;
        return w.set_focus();
    }
    WebviewWindowBuilder::new(app, MANAGER, WebviewUrl::App("index.html".into()))
        .title("G-routine 관리")
        .inner_size(760.0, 560.0)
        .min_inner_size(640.0, 480.0)
        .center()
        .disable_drag_drop_handler()
        .build()?;
    Ok(())
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != WIDGET {
        return;
    }
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = window.hide();
        }
        WindowEvent::Moved(pos) => {
            let h = window.outer_size().map(|s| s.height as i32).unwrap_or(0);
            schedule_position_save(window.app_handle(), pos.x, pos.y + h);
        }
        _ => {}
    }
}

/// 사용자가 끌어서 옮긴 위치만 저장한다 (프로그램 이동 직후 400ms는 무시, 500ms 디바운스).
fn schedule_position_save(app: &AppHandle, x: i32, bottom: i32) {
    let state = app.state::<AppState>();
    let recent = state
        .programmatic_move
        .lock()
        .ok()
        .and_then(|g| *g)
        .map(|t| t.elapsed() < Duration::from_millis(400))
        .unwrap_or(false);
    if recent {
        return;
    }
    let seq = state.move_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        let state = app.state::<AppState>();
        if state.move_seq.load(Ordering::SeqCst) == seq {
            let _ = state.with_conn(|c| settings::set_window_pos(c, x, bottom));
        }
    });
}

/// 30초마다 업무일을 확인해 바뀌면 백업 후 data-changed를 보낸다 (절전 복귀 포함).
pub fn spawn_day_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = None;
        loop {
            let state = app.state::<AppState>();
            if let Ok(day) = state.with_conn(|c| service::today(c, now())) {
                if last.is_some() && last != Some(day) {
                    if let Some(dir) = state.data_dir() {
                        let _ = state.with_conn(|c| startup::run_daily_backup(c, &dir, now()));
                    }
                    let _ = app.emit(DATA_CHANGED, ());
                }
                last = Some(day);
            }
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}
```

- [ ] **Step 6: `shell/tray.rs`**

```rust
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::db::settings;
use crate::model::Settings;
use crate::shell::{apply_setting_side_effects, window};
use crate::state::AppState;
use crate::DATA_CHANGED;

pub struct TrayChecks {
    top: CheckMenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
}

pub fn build(app: &AppHandle, s: Option<&Settings>) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "위젯 보이기").build(app)?;
    let manager = MenuItemBuilder::with_id("manager", "관리 창 열기").build(app)?;
    let top = CheckMenuItemBuilder::with_id("top", "항상 맨 위에 표시")
        .checked(s.map(|s| s.always_on_top).unwrap_or(false))
        .build(app)?;
    let autostart = CheckMenuItemBuilder::with_id("autostart", "컴퓨터 켜면 자동 시작")
        .checked(s.map(|s| s.autostart).unwrap_or(false))
        .build(app)?;
    let reset = MenuItemBuilder::with_id("reset", "위치 초기화").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "종료").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&manager)
        .separator()
        .item(&top)
        .item(&autostart)
        .separator()
        .item(&reset)
        .item(&quit)
        .build()?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("G-routine")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => window::show_widget(app),
            "manager" => {
                let _ = window::open_manager(app);
            }
            "top" => toggle(app, "always_on_top"),
            "autostart" => toggle(app, "autostart"),
            "reset" => window::reset_position(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                window::show_widget(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayChecks { top, autostart });
    Ok(())
}

pub fn sync_checks(app: &AppHandle, s: &Settings) {
    if let Some(c) = app.try_state::<TrayChecks>() {
        let _ = c.top.set_checked(s.always_on_top);
        let _ = c.autostart.set_checked(s.autostart);
    }
}

fn toggle(app: &AppHandle, key: &str) {
    let state = app.state::<AppState>();
    let result = state.with_conn(|c| {
        let s = settings::load(c)?;
        let current = if key == "always_on_top" { s.always_on_top } else { s.autostart };
        settings::apply(c, key, if current { "false" } else { "true" })?;
        settings::load(c)
    });
    match result {
        Ok(s) => {
            apply_setting_side_effects(app, key, &s);
            let _ = app.emit(DATA_CHANGED, ());
        }
        Err(_) => {
            if let Some(c) = app.try_state::<TrayChecks>() {
                let _ = c.top.set_checked(false);
                let _ = c.autostart.set_checked(false);
            }
        }
    }
}
```

- [ ] **Step 7: `shell/mod.rs` 완성**

```rust
pub mod position;
pub mod tray;
pub mod window;

use tauri::{AppHandle, Manager};

use crate::db::settings;
use crate::model::Settings;
use crate::state::AppState;

/// 부팅 직후: 트레이, 설정 반영, 위젯 배치 · 표시, 날짜 감시 시작
pub fn startup(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let s = state.with_conn(settings::load).ok();
    let saved = state.with_conn(settings::window_pos).ok().flatten();
    tray::build(app, s.as_ref())?;
    if let Some(s) = &s {
        apply_all(app, s);
    }
    window::place_widget(app, saved)?;
    window::show_widget(app);
    window::spawn_day_watcher(app.clone());
    Ok(())
}

/// 시작 설정 · 복구 · 백업 불러오기 직후 설정을 OS에 반영
pub fn after_ready(app: &AppHandle) {
    if let Ok(s) = app.state::<AppState>().with_conn(settings::load) {
        apply_all(app, &s);
    }
}

fn apply_all(app: &AppHandle, s: &Settings) {
    apply_always_on_top(app, s.always_on_top);
    sync_autostart(app, s.autostart);
    tray::sync_checks(app, s);
}

pub fn apply_setting_side_effects(app: &AppHandle, key: &str, s: &Settings) {
    match key {
        "always_on_top" => apply_always_on_top(app, s.always_on_top),
        "autostart" => sync_autostart(app, s.autostart),
        _ => {}
    }
    tray::sync_checks(app, s);
}

fn apply_always_on_top(app: &AppHandle, on: bool) {
    if let Some(w) = app.get_webview_window(window::WIDGET) {
        let _ = w.set_always_on_top(on);
    }
}

/// 자동 시작 등록(HKCU Run). 복원 프로그램에 지워졌으면 실행 때마다 다시 등록된다.
/// 디버그 빌드는 개발용 exe가 등록되지 않도록 건너뛴다.
fn sync_autostart(app: &AppHandle, on: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let enabled = launcher.is_enabled().unwrap_or(false);
    if on && !enabled {
        let _ = launcher.enable();
    } else if !on && enabled {
        let _ = launcher.disable();
    }
}
```

- [ ] **Step 8: `commands.rs`**

```rust
use std::path::Path;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::db::{routines, settings};
use crate::domain::day::fmt_ts;
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, Routine, RoutineInput, Settings, TodayView};
use crate::shell::{self, window};
use crate::state::{AppState, AppStatus};
use crate::storage::export;
use crate::{now, service, startup, DATA_CHANGED};

fn changed(app: &AppHandle) {
    let _ = app.emit(DATA_CHANGED, ());
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> AppStatus {
    state.status()
}

#[tauri::command]
pub fn setup(app: AppHandle, state: State<'_, AppState>, dir: String, template: String) -> AppResult<()> {
    startup::setup(&state, Path::new(&dir), &template, now())?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn restore_backup(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    startup::restore(&state, now())?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_today(state: State<'_, AppState>) -> AppResult<TodayView> {
    state.with_conn(|c| service::get_today(c, now()))
}

#[tauri::command]
pub fn set_done(app: AppHandle, state: State<'_, AppState>, item_id: i64, done: bool) -> AppResult<()> {
    state.with_conn(|c| service::set_done(c, item_id, done, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn quick_add(app: AppHandle, state: State<'_, AppState>, title: String) -> AppResult<()> {
    state.with_conn(|c| service::quick_add(c, &title, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_routines(state: State<'_, AppState>) -> AppResult<Vec<Routine>> {
    state.with_conn(|c| service::list_routines(c, now()))
}

#[tauri::command]
pub fn create_routine(app: AppHandle, state: State<'_, AppState>, input: RoutineInput) -> AppResult<i64> {
    let id = state.with_conn(|c| service::create_routine(c, input, now()))?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_routine(app: AppHandle, state: State<'_, AppState>, id: i64, input: RoutineInput) -> AppResult<()> {
    state.with_conn(|c| service::update_routine(c, id, input, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn archive_routine(app: AppHandle, state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|c| service::archive_routine(c, id, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_routines(app: AppHandle, state: State<'_, AppState>, ids: Vec<i64>) -> AppResult<()> {
    state.with_conn(|c| service::reorder_routines(c, &ids, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn history_month(state: State<'_, AppState>, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    state.with_conn(|c| service::history_month(c, year, month))
}

#[tauri::command]
pub fn history_day(state: State<'_, AppState>, day: String) -> AppResult<Vec<DayItem>> {
    state.with_conn(|c| service::history_day(c, &day))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.with_conn(settings::load)
}

#[tauri::command]
pub fn set_setting(app: AppHandle, state: State<'_, AppState>, key: String, value: String) -> AppResult<Settings> {
    let s = state.with_conn(|c| service::set_setting(c, &key, &value, now()))?;
    shell::apply_setting_side_effects(&app, &key, &s);
    changed(&app);
    Ok(s)
}

#[tauri::command]
pub fn open_link(app: AppHandle, state: State<'_, AppState>, routine_id: i64) -> AppResult<()> {
    let routine = state.with_conn(|c| routines::get(c, routine_id))?;
    let link = routine
        .link
        .filter(|l| routines::is_allowed_link(l))
        .ok_or_else(|| AppError::invalid("연결된 바로가기가 없어요"))?;
    let opener = app.opener();
    let result = if link.to_ascii_lowercase().starts_with("http") {
        opener.open_url(link, None::<&str>)
    } else {
        opener.open_path(link, None::<&str>)
    };
    result.map_err(|_| AppError::invalid("바로가기를 열 수 없어요. 주소나 경로를 확인해 주세요"))
}

#[tauri::command]
pub fn export_backup(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let backup = state.with_conn(|c| export::export(c, &fmt_ts(now())))?;
    export::write_file(Path::new(&path), &backup)
}

#[tauri::command]
pub fn import_backup(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let backup = export::read_file(Path::new(&path))?;
    state.with_conn(|c| {
        export::import(c, &backup)?;
        service::resync_today(c, now())
    })?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn change_data_dir(app: AppHandle, state: State<'_, AppState>, dir: String) -> AppResult<()> {
    startup::change_dir(&state, Path::new(&dir))?;
    changed(&app);
    Ok(())
}

/// Windows에서 동기 명령 안에서 창을 만들면 교착될 수 있어 async로 둔다.
#[tauri::command]
pub async fn open_manager(app: AppHandle) -> AppResult<()> {
    window::open_manager(&app)?;
    Ok(())
}

#[tauri::command]
pub fn resize_widget(app: AppHandle, height: f64) -> AppResult<()> {
    window::resize_widget(&app, height)?;
    Ok(())
}

#[tauri::command]
pub fn hide_widget(app: AppHandle) {
    window::hide_widget(&app);
}
```

- [ ] **Step 9: `lib.rs` 전체 교체**

```rust
mod commands;
mod db;
mod domain;
mod error;
mod model;
mod service;
mod shell;
mod startup;
mod state;
mod storage;
mod templates;
#[cfg(test)]
mod test_util;

use tauri::Manager;

pub const DATA_CHANGED: &str = "data-changed";

pub fn now() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            shell::window::show_widget(app);
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let location_file = app.path().app_config_dir()?.join("location.json");
            let exe_dir = std::env::current_exe()?
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            let state = state::AppState::new(location_file);
            startup::boot(&state, &exe_dir, &storage::location::drive_candidates(), now());
            app.manage(state);
            shell::startup(app.handle())?;
            Ok(())
        })
        .on_window_event(shell::window::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::setup,
            commands::restore_backup,
            commands::get_today,
            commands::set_done,
            commands::quick_add,
            commands::list_routines,
            commands::create_routine,
            commands::update_routine,
            commands::archive_routine,
            commands::reorder_routines,
            commands::history_month,
            commands::history_day,
            commands::get_settings,
            commands::set_setting,
            commands::open_link,
            commands::export_backup,
            commands::import_backup,
            commands::change_data_dir,
            commands::open_manager,
            commands::resize_widget,
            commands::hide_widget,
        ])
        .run(tauri::generate_context!())
        .expect("G-routine을 실행하지 못했어요");
}
```

- [ ] **Step 10: `tauri.conf.json` 교체**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "G-routine",
  "version": "0.1.0",
  "identifier": "com.groutine.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "widget",
        "title": "G-routine",
        "url": "index.html",
        "width": 280,
        "height": 360,
        "decorations": false,
        "transparent": true,
        "shadow": false,
        "resizable": false,
        "maximizable": false,
        "minimizable": false,
        "skipTaskbar": true,
        "alwaysOnTop": false,
        "visible": false,
        "focus": false,
        "dragDropEnabled": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "shortDescription": "교사를 위한 매일 루틴 위젯",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "languages": ["Korean"],
        "displayLanguageSelector": false
      }
    }
  }
}
```

- [ ] **Step 11: `capabilities/default.json` 교체**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "G-routine 위젯과 관리 창",
  "windows": ["widget", "manager"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "dialog:default"
  ]
}
```

- [ ] **Step 12: 전체 테스트와 빌드 확인**

Run: `cd src-tauri && cargo test` → Expected: 모든 테스트 PASS (약 45개). 경고는 허용, 오류는 0.
Run: `cargo build` → Expected: 성공.
Run: `cd .. && npm run build` → Expected: 성공 (Task 1의 임시 화면).

컴파일 오류가 Tauri API 시그니처 차이 때문이라면 `~/.cargo/registry/src/*/tauri-2.12.*/src`에서 해당 함수를 찾아 맞춘다. 동작(스펙)은 바꾸지 않는다.

- [ ] **Step 13: 커밋**

```bash
git add -A
git commit -m "feat(app): startup, widget/manager windows, tray, commands and Tauri config

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 7: 프런트엔드 기반 — 테마, UI 컴포넌트, API, 훅, 진입점, 미리보기용 가짜 백엔드

**Files:**
- Create: `src/lib/api.ts`, `src/lib/hooks.ts`, `src/lib/windowLabel.ts`, `src/components/ui/button.tsx`, `src/components/ui/input.tsx`, `src/components/ui/switch.tsx`, `src/components/ui/tabs.tsx`, `src/components/Chip.tsx`, `src/dev/mockBackend.ts`, `src/widget/WidgetApp.tsx`(임시), `src/manager/ManagerApp.tsx`(임시)
- Modify: `src/index.css`, `src/main.tsx`

**Interfaces:**
- Consumes: Task 6의 명령 · 이벤트 계약
- Produces:
  - `api` 객체 (아래 코드의 메서드 이름 그대로), 타입 `RepeatType, Routine, RoutineInput, DayItem, TodayView, DaySummary, Settings, ThemeName, AppStatus, SettingKey, TemplateName`, `errorMessage(e: unknown): string`
  - `useData<T>(load, deps?) -> { data: T | null, error, reload, setData }` (data-changed 이벤트와 창 focus 때 다시 불러옴)
  - `useSettings(enabled: boolean) -> { settings: Settings | null, update(key, value): Promise<Settings> }` (`<html data-theme>`도 갱신)
  - `useAutoResize<T extends HTMLElement>() -> RefObject<T | null>` (요소 높이를 `api.resizeWidget`으로 전달)
  - `Button`(variant: default | outline | ghost | soft | destructive, size: default | sm | icon | icon-sm), `Input`, `Switch`, `Tabs/TabsList/TabsTrigger/TabsContent`, `Chip`(kind: daily | weekdays | once | due)
  - Tailwind 색 유틸: `bg-background text-foreground bg-muted text-muted-foreground border-border border-input bg-primary text-primary-foreground bg-soft bg-soft-2 bg-soft-3 text-strong text-ink text-danger bg-danger-soft bg-success bg-chip-{daily,weekdays,once,due} text-chip-{…}-ink`

- [ ] **Step 1: 폰트 이름 확인**

Run: `grep -o "font-family:[^;]*" node_modules/pretendard/dist/web/variable/pretendardvariable.css | head -1`
Expected: `font-family:'Pretendard Variable'` 또는 비슷한 이름. 아래 CSS의 첫 글꼴 이름을 결과와 똑같이 맞춘다.

- [ ] **Step 2: `src/index.css`**

```css
@import "tailwindcss";

@theme {
  --font-sans: "Pretendard Variable", Pretendard, "Malgun Gothic", system-ui, sans-serif;
}

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--soft-3);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-soft: var(--soft);
  --color-soft-2: var(--soft-2);
  --color-soft-3: var(--soft-3);
  --color-strong: var(--strong);
  --color-ink: var(--ink);
  --color-danger: #a32d2d;
  --color-danger-soft: #fcebeb;
  --color-success: #5dcaa5;
  --color-chip-daily: #eeedfe;
  --color-chip-daily-ink: #3c3489;
  --color-chip-weekdays: #e1f5ee;
  --color-chip-weekdays-ink: #085041;
  --color-chip-once: #faece7;
  --color-chip-once-ink: #993c1d;
  --color-chip-due: #faeeda;
  --color-chip-due-ink: #854f0b;
}

:root {
  --background: #ffffff;
  --foreground: #2c2c2a;
  --muted: #f4f3ef;
  --muted-foreground: #888780;
  --border: #e8e6df;
  --input: #d3d1c7;
  --primary-foreground: #ffffff;
  /* lavender (기본) */
  --primary: #7f77dd;
  --soft: #eeedfe;
  --soft-2: #cecbf6;
  --soft-3: #afa9ec;
  --strong: #534ab7;
  --ink: #3c3489;
}

[data-theme="mint"] {
  --primary: #1d9e75;
  --soft: #e1f5ee;
  --soft-2: #9fe1cb;
  --soft-3: #5dcaa5;
  --strong: #0f6e56;
  --ink: #085041;
}

[data-theme="peach"] {
  --primary: #d85a30;
  --soft: #faece7;
  --soft-2: #f5c4b3;
  --soft-3: #f0997b;
  --strong: #993c1d;
  --ink: #712b13;
}

[data-theme="sky"] {
  --primary: #378add;
  --soft: #e6f1fb;
  --soft-2: #b5d4f4;
  --soft-3: #85b7eb;
  --strong: #185fa5;
  --ink: #0c447c;
}

[data-theme="lemon"] {
  --primary: #ba7517;
  --soft: #faeeda;
  --soft-2: #fac775;
  --soft-3: #ef9f27;
  --strong: #854f0b;
  --ink: #633806;
}

html,
body {
  margin: 0;
  background: transparent;
  color: var(--foreground);
  -webkit-font-smoothing: antialiased;
}

html[data-window="widget"] body {
  overflow: hidden;
  user-select: none;
}

html[data-window="manager"] body {
  background: var(--background);
}

html[data-preview="true"][data-window="widget"] #root {
  width: 280px;
  margin: 24px;
}
```

- [ ] **Step 3: `src/components/ui/*` (shadcn/ui 방식, 파스텔 토큰 적용)**

```tsx
// src/components/ui/button.tsx
import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg text-sm font-medium whitespace-nowrap transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-strong",
        outline: "border border-input bg-background hover:bg-muted",
        ghost: "text-foreground hover:bg-soft",
        soft: "bg-soft text-ink hover:bg-soft-2",
        destructive: "bg-danger text-white hover:opacity-90",
      },
      size: {
        default: "h-9 px-4",
        sm: "h-8 px-3 text-xs",
        icon: "size-8",
        "icon-sm": "size-6 rounded-md",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

function Button({
  className,
  variant,
  size,
  type = "button",
  ...props
}: React.ComponentProps<"button"> & VariantProps<typeof buttonVariants>) {
  return <button type={type} data-slot="button" className={cn(buttonVariants({ variant, size, className }))} {...props} />;
}

export { Button, buttonVariants };
```

```tsx
// src/components/ui/input.tsx
import * as React from "react";
import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "h-9 w-full min-w-0 rounded-lg border border-input bg-background px-3 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-soft-3 focus-visible:ring-2 focus-visible:ring-soft disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}

export { Input };
```

```tsx
// src/components/ui/switch.tsx
import * as React from "react";
import { Switch as SwitchPrimitive } from "radix-ui";
import { cn } from "@/lib/utils";

function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "peer inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-transparent transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-soft-3 data-[state=unchecked]:bg-input",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb className="pointer-events-none block size-4 rounded-full bg-white shadow-sm transition-transform data-[state=checked]:translate-x-[18px] data-[state=unchecked]:translate-x-0.5" />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
```

```tsx
// src/components/ui/tabs.tsx
import * as React from "react";
import { Tabs as TabsPrimitive } from "radix-ui";
import { cn } from "@/lib/utils";

function Tabs({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Root>) {
  return <TabsPrimitive.Root data-slot="tabs" className={cn("flex flex-col gap-4", className)} {...props} />;
}

function TabsList({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.List>) {
  return <TabsPrimitive.List data-slot="tabs-list" className={cn("inline-flex w-fit items-center gap-1 rounded-xl bg-muted p-1", className)} {...props} />;
}

function TabsTrigger({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      data-slot="tabs-trigger"
      className={cn(
        "rounded-lg px-3.5 py-1.5 text-sm text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[state=active]:bg-background data-[state=active]:font-medium data-[state=active]:text-ink data-[state=active]:shadow-sm",
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return <TabsPrimitive.Content data-slot="tabs-content" className={cn("outline-none", className)} {...props} />;
}

export { Tabs, TabsList, TabsTrigger, TabsContent };
```

```tsx
// src/components/Chip.tsx
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export type ChipKind = "daily" | "weekdays" | "once" | "due";

const STYLES: Record<ChipKind, string> = {
  daily: "bg-chip-daily text-chip-daily-ink",
  weekdays: "bg-chip-weekdays text-chip-weekdays-ink",
  once: "bg-chip-once text-chip-once-ink",
  due: "bg-chip-due text-chip-due-ink",
};

export function Chip({ kind, children, className }: { kind: ChipKind; children: ReactNode; className?: string }) {
  return (
    <span className={cn("inline-flex shrink-0 items-center rounded-full px-2 py-px text-[11px] leading-4", STYLES[kind], className)}>
      {children}
    </span>
  );
}
```

- [ ] **Step 4: `src/lib/api.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";

export type RepeatType = "daily" | "weekdays" | "once";
export type ThemeName = "lavender" | "mint" | "peach" | "sky" | "lemon";
export type SettingKey = "always_on_top" | "autostart" | "hide_weekends" | "day_start_hour" | "theme";
export type TemplateName = "homeroom" | "subject" | "empty";

export interface Routine {
  id: number;
  title: string;
  repeatType: RepeatType;
  weekdays: number;
  onceDate: string | null;
  dueTime: string | null;
  link: string | null;
  sortOrder: number;
  createdAt: string;
  archivedAt: string | null;
}

export interface RoutineInput {
  title: string;
  repeatType: RepeatType;
  weekdays: number;
  onceDate: string | null;
  dueTime: string | null;
  link: string | null;
}

export interface DayItem {
  id: number;
  day: string;
  routineId: number;
  title: string;
  sortOrder: number;
  completedAt: string | null;
  repeatType: RepeatType;
  dueTime: string | null;
  hasLink: boolean;
}

export interface TodayView {
  day: string;
  weekendHidden: boolean;
  pending: DayItem[];
  done: DayItem[];
}

export interface DaySummary {
  day: string;
  total: number;
  completed: number;
}

export interface Settings {
  alwaysOnTop: boolean;
  autostart: boolean;
  hideWeekends: boolean;
  dayStartHour: number;
  theme: ThemeName;
}

export interface AppStatus {
  ready: boolean;
  corrupt: boolean;
  portable: boolean;
  previousDir: string | null;
  suggestedDir: string;
  dataDir: string | null;
}

export const api = {
  status: () => invoke<AppStatus>("get_status"),
  setup: (dir: string, template: TemplateName) => invoke<void>("setup", { dir, template }),
  restoreBackup: () => invoke<void>("restore_backup"),
  today: () => invoke<TodayView>("get_today"),
  setDone: (itemId: number, done: boolean) => invoke<void>("set_done", { itemId, done }),
  quickAdd: (title: string) => invoke<void>("quick_add", { title }),
  listRoutines: () => invoke<Routine[]>("list_routines"),
  createRoutine: (input: RoutineInput) => invoke<number>("create_routine", { input }),
  updateRoutine: (id: number, input: RoutineInput) => invoke<void>("update_routine", { id, input }),
  archiveRoutine: (id: number) => invoke<void>("archive_routine", { id }),
  reorderRoutines: (ids: number[]) => invoke<void>("reorder_routines", { ids }),
  historyMonth: (year: number, month: number) => invoke<DaySummary[]>("history_month", { year, month }),
  historyDay: (day: string) => invoke<DayItem[]>("history_day", { day }),
  settings: () => invoke<Settings>("get_settings"),
  setSetting: (key: SettingKey, value: string) => invoke<Settings>("set_setting", { key, value }),
  openLink: (routineId: number) => invoke<void>("open_link", { routineId }),
  exportBackup: (path: string) => invoke<void>("export_backup", { path }),
  importBackup: (path: string) => invoke<void>("import_backup", { path }),
  changeDataDir: (dir: string) => invoke<void>("change_data_dir", { dir }),
  openManager: () => invoke<void>("open_manager"),
  resizeWidget: (height: number) => invoke<void>("resize_widget", { height }),
  hideWidget: () => invoke<void>("hide_widget"),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "알 수 없는 오류가 생겼어요. 다시 시도해 주세요";
}
```

- [ ] **Step 5: `src/lib/hooks.ts`**

```ts
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorMessage, type SettingKey, type Settings } from "@/lib/api";

export const DATA_CHANGED = "data-changed";

/** 불러오기 + data-changed 이벤트 · 창 focus 때 자동 새로고침 */
export function useData<T>(load: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const loadRef = useRef(load);
  loadRef.current = load;

  const reload = useCallback(async () => {
    try {
      setData(await loadRef.current());
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reload, ...deps]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen(DATA_CHANGED, () => void reload())
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    const onFocus = () => void reload();
    window.addEventListener("focus", onFocus);
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener("focus", onFocus);
    };
  }, [reload]);

  return { data, error, reload, setData };
}

export function useSettings(enabled: boolean) {
  const { data, setData } = useData<Settings | null>(
    () => (enabled ? api.settings() : Promise.resolve(null)),
    [enabled],
  );
  const theme = data?.theme ?? "lavender";
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  const update = useCallback(
    async (key: SettingKey, value: string) => {
      const next = await api.setSetting(key, value);
      setData(next);
      return next;
    },
    [setData],
  );
  return { settings: data, update };
}

/** 위젯 내용 높이를 Rust에 알려 창 높이를 맞춘다 (아래 모서리 고정). */
export function useAutoResize<T extends HTMLElement>() {
  const ref = useRef<T | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    let last = 0;
    const ro = new ResizeObserver(() => {
      const h = Math.ceil(el.getBoundingClientRect().height);
      if (h !== last) {
        last = h;
        api.resizeWidget(h).catch(() => {});
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return ref;
}
```

- [ ] **Step 6: `src/lib/windowLabel.ts`, 임시 앱 컴포넌트, `src/main.tsx`**

```ts
// src/lib/windowLabel.ts
import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowLabel = "widget" | "manager";

export function currentWindowLabel(): WindowLabel {
  let label = new URLSearchParams(window.location.search).get("window") ?? "widget";
  try {
    label = getCurrentWindow().label;
  } catch {
    // 브라우저 미리보기: 쿼리 값 사용
  }
  return label === "manager" ? "manager" : "widget";
}
```

```tsx
// src/widget/WidgetApp.tsx (Task 8에서 교체)
export function WidgetApp() {
  return <div className="p-4 text-sm">위젯</div>;
}
```

```tsx
// src/manager/ManagerApp.tsx (Task 9에서 교체)
export function ManagerApp() {
  return <div className="p-6 text-sm">관리 창</div>;
}
```

```tsx
// src/main.tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import "pretendard/dist/web/variable/pretendardvariable.css";
import "./index.css";
import { currentWindowLabel } from "@/lib/windowLabel";
import { WidgetApp } from "@/widget/WidgetApp";
import { ManagerApp } from "@/manager/ManagerApp";

async function start() {
  if (import.meta.env.DEV && !isTauri()) {
    const { installMockBackend } = await import("@/dev/mockBackend");
    installMockBackend();
    document.documentElement.dataset.preview = "true";
  }
  const label = currentWindowLabel();
  document.documentElement.dataset.window = label;
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>{label === "manager" ? <ManagerApp /> : <WidgetApp />}</React.StrictMode>,
  );
}

void start();
```

- [ ] **Step 7: `src/dev/mockBackend.ts` (브라우저 미리보기 전용, DEV 빌드에서만 import)**

`npm run dev` 후 브라우저에서 `http://localhost:1420/`(위젯), `?window=manager`(관리 창), `?setup=1`(첫 실행 화면)로 화면을 확인하는 용도다. 데이터는 메모리에만 있다.

```ts
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AppStatus, DayItem, DaySummary, Routine, RoutineInput, Settings, TodayView } from "@/lib/api";

const pad = (n: number) => String(n).padStart(2, "0");
const dayStr = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const stamp = (d: Date) => `${dayStr(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;

const TODAY = dayStr(new Date());
let nextId = 100;

function routine(id: number, title: string, extra: Partial<Routine> = {}): Routine {
  return {
    id,
    title,
    repeatType: "daily",
    weekdays: 0,
    onceDate: null,
    dueTime: null,
    link: null,
    sortOrder: id,
    createdAt: `${TODAY}T08:00:00`,
    archivedAt: null,
    ...extra,
  };
}

const routines: Routine[] = [
  routine(1, "출결 확인", { dueTime: "09:00", link: "https://www.neis.go.kr" }),
  routine(2, "수업 준비"),
  routine(3, "누가기록 작성", { link: "https://www.neis.go.kr" }),
  routine(4, "공문 확인", { link: "https://www.google.com" }),
  routine(5, "알림장 작성"),
  routine(6, "주간학습안내 배부", { repeatType: "weekdays", weekdays: 16 }),
  routine(7, "가정통신문 회수", { repeatType: "once", onceDate: TODAY }),
];
const doneAt = new Map<number, string>([[2, `${TODAY}T08:55:00`]]);
let settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: false, dayStartHour: 4, theme: "lavender" };
let ready = new URLSearchParams(window.location.search).get("setup") !== "1";
const DATA_DIR = "D:\\G-routine\\data";

function bit(day: string) {
  const [y, m, d] = day.split("-").map(Number);
  const wd = new Date(y, m - 1, d).getDay(); // 0=일
  return 1 << ((wd + 6) % 7);
}

function applies(r: Routine, day: string) {
  if (r.archivedAt) return false;
  if (r.repeatType === "daily") return true;
  if (r.repeatType === "weekdays") return (r.weekdays & bit(day)) !== 0;
  return r.onceDate === day;
}

function toItem(r: Routine, day: string, completedAt: string | null): DayItem {
  return {
    id: r.id,
    day,
    routineId: r.id,
    title: r.title,
    sortOrder: r.sortOrder,
    completedAt,
    repeatType: r.repeatType,
    dueTime: r.dueTime,
    hasLink: Boolean(r.link),
  };
}

function today(): TodayView {
  const items = routines
    .filter((r) => applies(r, TODAY))
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map((r) => toItem(r, TODAY, doneAt.get(r.id) ?? null));
  return {
    day: TODAY,
    weekendHidden: false,
    pending: items.filter((i) => !i.completedAt),
    done: items.filter((i) => i.completedAt),
  };
}

function historyMonth(year: number, month: number): DaySummary[] {
  const out: DaySummary[] = [];
  const days = new Date(year, month, 0).getDate();
  for (let d = 1; d <= days; d++) {
    const day = `${year}-${pad(month)}-${pad(d)}`;
    if (day > TODAY) break;
    const wd = new Date(year, month - 1, d).getDay();
    if (wd === 0 || wd === 6) continue;
    if (day === TODAY) {
      const v = today();
      out.push({ day, total: v.pending.length + v.done.length, completed: v.done.length });
    } else {
      out.push({ day, total: 6, completed: [6, 4, 6, 2, 5, 0][d % 6] });
    }
  }
  return out;
}

function historyDay(day: string): DayItem[] {
  if (day === TODAY) {
    const v = today();
    return [...v.pending, ...v.done].sort((a, b) => a.sortOrder - b.sortOrder);
  }
  const summary = historyMonth(Number(day.slice(0, 4)), Number(day.slice(5, 7))).find((s) => s.day === day);
  if (!summary) return [];
  return routines
    .filter((r) => r.repeatType === "daily")
    .slice(0, summary.total)
    .map((r, i) => toItem(r, day, i < summary.completed ? `${day}T0${8 + Math.floor(i / 3)}:${pad(10 + i * 7)}:00` : null));
}

function status(): AppStatus {
  return { ready, corrupt: false, portable: false, previousDir: null, suggestedDir: DATA_DIR, dataDir: ready ? DATA_DIR : null };
}

function changed() {
  setTimeout(() => window.dispatchEvent(new Event("focus")), 0);
}

function fromInput(id: number, input: RoutineInput, sortOrder: number): Routine {
  return routine(id, input.title, { ...input, sortOrder });
}

const KEY_MAP: Record<string, keyof Settings> = {
  always_on_top: "alwaysOnTop",
  autostart: "autostart",
  hide_weekends: "hideWeekends",
  day_start_hour: "dayStartHour",
  theme: "theme",
};

export function installMockBackend() {
  const label = new URLSearchParams(window.location.search).get("window") ?? "widget";
  mockWindows(label);
  mockIPC((cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    if (cmd.startsWith("plugin:event|")) return 0;
    if (cmd.startsWith("plugin:dialog|")) return null;
    switch (cmd) {
      case "get_status":
        return status();
      case "setup":
        ready = true;
        changed();
        return null;
      case "get_today":
        return today();
      case "set_done": {
        const id = args.itemId as number;
        if (args.done) doneAt.set(id, stamp(new Date()));
        else doneAt.delete(id);
        changed();
        return null;
      }
      case "quick_add":
        routines.push(routine(nextId++, String(args.title), { repeatType: "once", onceDate: TODAY, sortOrder: 999 }));
        changed();
        return null;
      case "list_routines":
        return routines.filter((r) => !r.archivedAt && !(r.repeatType === "once" && (r.onceDate ?? "") < TODAY));
      case "create_routine": {
        const id = nextId++;
        routines.push(fromInput(id, args.input as RoutineInput, routines.length));
        changed();
        return id;
      }
      case "update_routine": {
        const idx = routines.findIndex((r) => r.id === args.id);
        if (idx >= 0) routines[idx] = fromInput(routines[idx].id, args.input as RoutineInput, routines[idx].sortOrder);
        changed();
        return null;
      }
      case "archive_routine": {
        const r = routines.find((x) => x.id === args.id);
        if (r) r.archivedAt = stamp(new Date());
        changed();
        return null;
      }
      case "reorder_routines":
        (args.ids as number[]).forEach((id, i) => {
          const r = routines.find((x) => x.id === id);
          if (r) r.sortOrder = i;
        });
        changed();
        return null;
      case "history_month":
        return historyMonth(args.year as number, args.month as number);
      case "history_day":
        return historyDay(args.day as string);
      case "get_settings":
        return settings;
      case "set_setting": {
        const field = KEY_MAP[args.key as string];
        const raw = args.value as string;
        const value = field === "dayStartHour" ? Number(raw) : field === "theme" ? raw : raw === "true";
        settings = { ...settings, [field]: value } as Settings;
        changed();
        return settings;
      }
      case "open_link":
        console.info("[mock] open_link", args.routineId);
        return null;
      case "open_manager":
        window.open("?window=manager", "_blank");
        return null;
      default:
        return null;
    }
  });
}
```

- [ ] **Step 8: 빌드 · 테스트 확인과 커밋**

Run: `npm run build` → Expected: 타입 오류 없이 성공
Run: `npm test` → Expected: PASS

```bash
git add -A
git commit -m "feat(ui): theme tokens, shadcn-style components, API client, hooks and preview backend

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: 위젯 창 — 첫 실행 설정, 오늘의 루틴, 체크 · 되돌리기, 빠른 추가

**Files:**
- Create: `src/widget/today.ts`, `src/widget/today.test.ts`, `src/widget/TaskRow.tsx`, `src/widget/DoneRow.tsx`, `src/widget/UndoToast.tsx`, `src/widget/Notice.tsx`, `src/widget/QuickAdd.tsx`, `src/widget/WidgetHeader.tsx`, `src/widget/TodayPanel.tsx`, `src/widget/TodayPanel.test.tsx`, `src/widget/Setup.tsx`
- Modify: `src/widget/WidgetApp.tsx` (교체)

**Interfaces:**
- Consumes: Task 7의 `api`, `useData`, `useSettings`, `useAutoResize`, `Button`, `Chip`, `cn`
- Produces:
  - `today.ts`: `formatDayLabel(day: string): string`, `progress(v: TodayView): { done: number; total: number; percent: number }`, `markDone(v, itemId, at): TodayView`, `markPending(v, itemId): TodayView`, `localTimestamp(d?: Date): string`, `formatTime(ts: string | null): string`
  - `TodayPanel({ pinned: boolean; onTogglePin: () => void })`

- [ ] **Step 1: `today.test.ts` (실패하는 테스트)**

```ts
import type { DayItem, TodayView } from "@/lib/api";
import { formatDayLabel, formatTime, localTimestamp, markDone, markPending, progress } from "./today";

const item = (id: number, title: string, extra: Partial<DayItem> = {}): DayItem => ({
  id,
  day: "2026-10-05",
  routineId: id,
  title,
  sortOrder: id,
  completedAt: null,
  repeatType: "daily",
  dueTime: null,
  hasLink: false,
  ...extra,
});

const view: TodayView = { day: "2026-10-05", weekendHidden: false, pending: [item(1, "A"), item(2, "B")], done: [] };

test("formats Korean day label", () => {
  expect(formatDayLabel("2026-10-05")).toBe("10월 5일 월요일");
  expect(formatDayLabel("2026-10-11")).toBe("10월 11일 일요일");
});

test("progress counts done over total", () => {
  expect(progress(view)).toEqual({ done: 0, total: 2, percent: 0 });
  expect(progress(markDone(view, 1, "2026-10-05T08:00:00"))).toEqual({ done: 1, total: 2, percent: 50 });
  expect(progress({ ...view, pending: [] })).toEqual({ done: 0, total: 0, percent: 0 });
});

test("markDone and markPending move items keeping sort order", () => {
  const done = markDone(view, 1, "2026-10-05T08:00:00");
  expect(done.pending.map((i) => i.id)).toEqual([2]);
  expect(done.done[0]).toMatchObject({ id: 1, completedAt: "2026-10-05T08:00:00" });
  const back = markPending(done, 1);
  expect(back.pending.map((i) => i.id)).toEqual([1, 2]);
  expect(back.done).toEqual([]);
  expect(markDone(view, 99, "x")).toBe(view);
});

test("time helpers", () => {
  expect(localTimestamp(new Date(2026, 9, 5, 8, 7, 3))).toBe("2026-10-05T08:07:03");
  expect(formatTime("2026-10-05T08:47:12")).toBe("08:47");
  expect(formatTime(null)).toBe("");
});
```

Run: `npx vitest run src/widget/today.test.ts` → Expected: FAIL (모듈 없음)

- [ ] **Step 2: `today.ts` 구현**

```ts
import type { TodayView } from "@/lib/api";

const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];
const pad = (n: number) => String(n).padStart(2, "0");

export function formatDayLabel(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  const wd = new Date(y, m - 1, d).getDay();
  return `${m}월 ${d}일 ${WEEKDAYS[wd]}요일`;
}

export function progress(v: TodayView) {
  const done = v.done.length;
  const total = done + v.pending.length;
  return { done, total, percent: total === 0 ? 0 : Math.round((done / total) * 100) };
}

export function markDone(v: TodayView, itemId: number, at: string): TodayView {
  const item = v.pending.find((i) => i.id === itemId);
  if (!item) return v;
  return {
    ...v,
    pending: v.pending.filter((i) => i.id !== itemId),
    done: [...v.done, { ...item, completedAt: at }],
  };
}

export function markPending(v: TodayView, itemId: number): TodayView {
  const item = v.done.find((i) => i.id === itemId);
  if (!item) return v;
  const pending = [...v.pending, { ...item, completedAt: null }].sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id);
  return { ...v, pending, done: v.done.filter((i) => i.id !== itemId) };
}

export function localTimestamp(d: Date = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export function formatTime(ts: string | null): string {
  return ts ? ts.slice(11, 16) : "";
}
```

Run: `npx vitest run src/widget/today.test.ts` → Expected: PASS (4 tests)

- [ ] **Step 3: 작은 컴포넌트들**

```tsx
// src/widget/TaskRow.tsx
import { Check, ExternalLink } from "lucide-react";
import type { DayItem } from "@/lib/api";
import { Chip } from "@/components/Chip";
import { cn } from "@/lib/utils";

interface Props {
  item: DayItem;
  leaving: boolean;
  onCheck: () => void;
  onOpenLink: () => void;
}

export function TaskRow({ item, leaving, onCheck, onOpenLink }: Props) {
  return (
    <li
      className={cn(
        "flex items-center gap-2.5 rounded-lg px-2.5 py-2 transition-all duration-200 hover:bg-soft/60",
        leaving && "-translate-x-2 opacity-0",
      )}
    >
      <button
        type="button"
        aria-label={`${item.title} 완료`}
        onClick={onCheck}
        className={cn(
          "flex size-[18px] shrink-0 items-center justify-center rounded-full border-[1.5px] border-soft-3 bg-background transition-colors hover:bg-soft",
          leaving && "bg-soft-3",
        )}
      >
        {leaving && <Check className="size-3 text-white" strokeWidth={3} />}
      </button>
      <span className="min-w-0 flex-1 truncate text-[13px]">{item.title}</span>
      {item.repeatType === "once" && <Chip kind="once">오늘만</Chip>}
      {item.dueTime && <Chip kind="due">{item.dueTime}</Chip>}
      {item.hasLink && (
        <button
          type="button"
          aria-label={`${item.title} 바로가기 열기`}
          onClick={onOpenLink}
          className="rounded p-0.5 text-primary hover:bg-soft"
        >
          <ExternalLink className="size-3.5" />
        </button>
      )}
    </li>
  );
}
```

```tsx
// src/widget/DoneRow.tsx
import { Check } from "lucide-react";
import type { DayItem } from "@/lib/api";
import { formatTime } from "./today";

export function DoneRow({ item, onUncheck }: { item: DayItem; onUncheck: () => void }) {
  return (
    <li className="flex items-center gap-2.5 rounded-lg px-2.5 py-1.5 hover:bg-muted">
      <button
        type="button"
        aria-label={`${item.title} 완료 취소`}
        onClick={onUncheck}
        className="flex size-[18px] shrink-0 items-center justify-center rounded-full bg-soft-3"
      >
        <Check className="size-3 text-white" strokeWidth={3} />
      </button>
      <span className="min-w-0 flex-1 truncate text-[13px] text-muted-foreground line-through">{item.title}</span>
      <span className="text-[11px] text-muted-foreground">{formatTime(item.completedAt)}</span>
    </li>
  );
}
```

```tsx
// src/widget/UndoToast.tsx
import { useEffect } from "react";

export function UndoToast({ message, onUndo, onClose }: { message: string; onUndo: () => void; onClose: () => void }) {
  useEffect(() => {
    const t = setTimeout(onClose, 3000);
    return () => clearTimeout(t);
  }, [message, onClose]);
  return (
    <div role="status" className="mx-2 mt-1.5 flex items-center justify-between rounded-lg bg-foreground px-3 py-1.5 text-xs text-white">
      <span className="truncate">{message}</span>
      <button type="button" onClick={onUndo} className="ml-2 shrink-0 font-medium text-soft-2 hover:underline">
        되돌리기
      </button>
    </div>
  );
}
```

```tsx
// src/widget/Notice.tsx
import { useEffect } from "react";

export function Notice({ message, onClose }: { message: string; onClose: () => void }) {
  useEffect(() => {
    const t = setTimeout(onClose, 4000);
    return () => clearTimeout(t);
  }, [message, onClose]);
  return (
    <p role="alert" className="mx-2 mt-1.5 rounded-lg bg-danger-soft px-3 py-1.5 text-xs text-danger">
      {message}
    </p>
  );
}
```

```tsx
// src/widget/QuickAdd.tsx
import { useState } from "react";
import { Plus } from "lucide-react";

export function QuickAdd({ onAdd }: { onAdd: (title: string) => Promise<void> }) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit() {
    const title = value.trim();
    if (!title || busy) return;
    setBusy(true);
    try {
      await onAdd(title);
      setValue("");
    } catch {
      // 오류 문구는 부모가 보여준다
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="px-2.5 pt-1.5 pb-2.5">
      <label className="flex items-center gap-1.5 rounded-lg border border-dashed border-soft-2 px-2.5 py-1.5 text-xs text-muted-foreground focus-within:border-soft-3">
        <Plus className="size-3.5 shrink-0" />
        <input
          aria-label="오늘 할 일 추가"
          value={value}
          maxLength={40}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) {
              e.preventDefault();
              void submit();
            }
          }}
          placeholder="오늘 할 일 추가 (Enter)"
          className="min-w-0 flex-1 bg-transparent text-foreground outline-none placeholder:text-muted-foreground"
        />
      </label>
    </div>
  );
}
```

```tsx
// src/widget/WidgetHeader.tsx
import { Minus, Pin, PinOff, Settings as SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";

export function WidgetHeader({ dayLabel, pinned, onTogglePin }: { dayLabel: string; pinned: boolean; onTogglePin: () => void }) {
  return (
    <div data-tauri-drag-region className="flex cursor-default items-center justify-between px-3.5 pt-3 pb-2.5">
      <div data-tauri-drag-region>
        <div data-tauri-drag-region className="text-sm font-semibold">
          {dayLabel}
        </div>
        <div data-tauri-drag-region className="text-[11px] text-muted-foreground">
          G-routine
        </div>
      </div>
      <div className="flex items-center gap-0.5">
        <Button variant="ghost" size="icon-sm" aria-label={pinned ? "맨 위 고정 해제" : "맨 위에 고정"} onClick={onTogglePin}>
          {pinned ? <Pin className="size-3.5 text-primary" /> : <PinOff className="size-3.5 text-muted-foreground" />}
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label="관리 창 열기" onClick={() => void api.openManager()}>
          <SettingsIcon className="size-3.5 text-muted-foreground" />
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label="위젯 숨기기" onClick={() => void api.hideWidget()}>
          <Minus className="size-3.5 text-muted-foreground" />
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: `TodayPanel.test.tsx` (실패하는 테스트)**

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { DayItem, TodayView } from "@/lib/api";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const apiMock = vi.hoisted(() => ({
  today: vi.fn(),
  setDone: vi.fn(),
  quickAdd: vi.fn(),
  openLink: vi.fn(),
  openManager: vi.fn(),
  hideWidget: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { TodayPanel } from "./TodayPanel";

const item = (id: number, title: string, extra: Partial<DayItem> = {}): DayItem => ({
  id,
  day: "2026-10-05",
  routineId: id + 10,
  title,
  sortOrder: id,
  completedAt: null,
  repeatType: "daily",
  dueTime: null,
  hasLink: false,
  ...extra,
});

const view: TodayView = {
  day: "2026-10-05",
  weekendHidden: false,
  pending: [item(1, "출결 확인", { hasLink: true, dueTime: "09:00" }), item(2, "가정통신문 회수", { repeatType: "once" })],
  done: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  apiMock.today.mockResolvedValue(JSON.parse(JSON.stringify(view)));
  apiMock.setDone.mockResolvedValue(undefined);
  apiMock.quickAdd.mockResolvedValue(undefined);
  apiMock.openLink.mockResolvedValue(undefined);
});

const renderPanel = () => render(<TodayPanel pinned onTogglePin={() => {}} />);

test("shows date, progress and chips", async () => {
  renderPanel();
  expect(await screen.findByText("10월 5일 월요일")).toBeInTheDocument();
  expect(screen.getByTestId("progress-count")).toHaveTextContent("0 / 2");
  expect(screen.getByText("오늘만")).toBeInTheDocument();
  expect(screen.getByText("09:00")).toBeInTheDocument();
});

test("checking hides the item and undo brings it back", async () => {
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 완료" }));
  await waitFor(() => expect(screen.queryByRole("button", { name: "출결 확인 완료" })).not.toBeInTheDocument());
  expect(apiMock.setDone).toHaveBeenCalledWith(1, true);
  expect(screen.getByTestId("progress-count")).toHaveTextContent("1 / 2");

  await user.click(screen.getByRole("button", { name: "되돌리기" }));
  expect(await screen.findByRole("button", { name: "출결 확인 완료" })).toBeInTheDocument();
  expect(apiMock.setDone).toHaveBeenLastCalledWith(1, false);
});

test("failed save keeps the item and shows the error", async () => {
  apiMock.setDone.mockRejectedValueOnce("데이터를 저장하거나 읽지 못했어요");
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 완료" }));
  expect(await screen.findByText("데이터를 저장하거나 읽지 못했어요")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "출결 확인 완료" })).toBeInTheDocument();
});

test("quick add sends trimmed text on Enter", async () => {
  const user = userEvent.setup();
  renderPanel();
  const input = await screen.findByLabelText("오늘 할 일 추가");
  await user.type(input, "  학부모 상담 전화  {Enter}");
  expect(apiMock.quickAdd).toHaveBeenCalledWith("학부모 상담 전화");
  await waitFor(() => expect(input).toHaveValue(""));
});

test("link button opens the routine link", async () => {
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 바로가기 열기" }));
  expect(apiMock.openLink).toHaveBeenCalledWith(11);
});

test("all done shows a finished message", async () => {
  apiMock.today.mockResolvedValue({ ...view, pending: [], done: [item(1, "출결 확인", { completedAt: "2026-10-05T08:47:00" })] });
  renderPanel();
  expect(await screen.findByText("오늘 루틴을 모두 마쳤어요")).toBeInTheDocument();
  expect(screen.getByTestId("progress-count")).toHaveTextContent("모두 완료");
});
```

Run: `npx vitest run src/widget/TodayPanel.test.tsx` → Expected: FAIL (모듈 없음)

- [ ] **Step 5: `TodayPanel.tsx` 구현**

```tsx
import { useCallback, useState } from "react";
import { ChevronDown, ChevronUp, Coffee, PartyPopper } from "lucide-react";
import { api, errorMessage, type DayItem, type TodayView } from "@/lib/api";
import { useData } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { DoneRow } from "./DoneRow";
import { Notice } from "./Notice";
import { QuickAdd } from "./QuickAdd";
import { TaskRow } from "./TaskRow";
import { UndoToast } from "./UndoToast";
import { WidgetHeader } from "./WidgetHeader";
import { formatDayLabel, localTimestamp, markDone, markPending, progress } from "./today";

const LEAVE_MS = 220;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function EmptyState({ view, total }: { view: TodayView; total: number }) {
  if (total > 0) {
    return (
      <div className="flex flex-col items-center gap-1 px-4 py-5 text-center text-xs text-muted-foreground">
        <PartyPopper className="size-5 text-primary" />
        <span className="font-medium text-foreground">오늘 루틴을 모두 마쳤어요</span>
      </div>
    );
  }
  if (view.weekendHidden) {
    return (
      <div className="flex flex-col items-center gap-1 px-4 py-5 text-center text-xs text-muted-foreground">
        <Coffee className="size-5 text-primary" />
        <span className="font-medium text-foreground">좋은 주말 보내세요</span>
      </div>
    );
  }
  return (
    <div className="px-4 py-5 text-center text-xs leading-5 text-muted-foreground">
      등록된 루틴이 없어요.
      <br />
      ⚙ 관리 창에서 추가하거나 아래에 적어 보세요.
    </div>
  );
}

export function TodayPanel({ pinned, onTogglePin }: { pinned: boolean; onTogglePin: () => void }) {
  const { data, setData } = useData(api.today);
  const [leaving, setLeaving] = useState<number[]>([]);
  const [undoItem, setUndoItem] = useState<DayItem | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [showDone, setShowDone] = useState(false);
  const closeUndo = useCallback(() => setUndoItem(null), []);
  const closeNotice = useCallback(() => setNotice(null), []);

  if (!data) return <div className="h-28" />;
  const view = data;
  const { done, total, percent } = progress(view);
  const allDone = total > 0 && done === total;

  async function check(item: DayItem) {
    if (leaving.includes(item.id)) return;
    setLeaving((l) => [...l, item.id]);
    await sleep(LEAVE_MS);
    try {
      await api.setDone(item.id, true);
      setData((v) => (v ? markDone(v, item.id, localTimestamp()) : v));
      setUndoItem(item);
    } catch (e) {
      setNotice(errorMessage(e));
    } finally {
      setLeaving((l) => l.filter((id) => id !== item.id));
    }
  }

  async function uncheck(item: DayItem) {
    try {
      await api.setDone(item.id, false);
      setData((v) => (v ? markPending(v, item.id) : v));
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }

  async function openLink(item: DayItem) {
    try {
      await api.openLink(item.routineId);
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }

  async function quickAdd(title: string) {
    try {
      await api.quickAdd(title);
    } catch (e) {
      setNotice(errorMessage(e));
      throw e;
    }
  }

  return (
    <div className="flex flex-col">
      <WidgetHeader dayLabel={formatDayLabel(view.day)} pinned={pinned} onTogglePin={onTogglePin} />
      <div className="px-3.5 pb-2.5">
        <div className="mb-1 flex justify-between text-xs text-muted-foreground">
          <span>오늘의 루틴</span>
          <span data-testid="progress-count" className="font-medium text-strong">
            {allDone ? "모두 완료" : `${done} / ${total}`}
          </span>
        </div>
        <div className="h-1.5 rounded-full bg-soft">
          <div
            className={cn("h-1.5 rounded-full transition-all duration-300", allDone ? "bg-success" : "bg-soft-3")}
            style={{ width: `${percent}%` }}
          />
        </div>
      </div>
      {view.pending.length > 0 ? (
        <ul className="max-h-[252px] overflow-y-auto px-1">
          {view.pending.map((item) => (
            <TaskRow
              key={item.id}
              item={item}
              leaving={leaving.includes(item.id)}
              onCheck={() => void check(item)}
              onOpenLink={() => void openLink(item)}
            />
          ))}
        </ul>
      ) : (
        <EmptyState view={view} total={total} />
      )}
      {undoItem && (
        <UndoToast
          message={`${undoItem.title} 완료`}
          onUndo={() => {
            const it = undoItem;
            setUndoItem(null);
            void uncheck(it);
          }}
          onClose={closeUndo}
        />
      )}
      {notice && <Notice message={notice} onClose={closeNotice} />}
      {view.done.length > 0 && (
        <div className="mt-2 border-t border-border">
          <button
            type="button"
            onClick={() => setShowDone((s) => !s)}
            className="flex w-full items-center gap-1 px-3.5 py-2 text-xs text-muted-foreground hover:text-foreground"
          >
            {showDone ? <ChevronUp className="size-3.5" /> : <ChevronDown className="size-3.5" />}
            완료 {view.done.length}개 {showDone ? "숨기기" : "보기"}
          </button>
          {showDone && (
            <ul className="max-h-[160px] overflow-y-auto px-1 pb-1">
              {view.done.map((item) => (
                <DoneRow key={item.id} item={item} onUncheck={() => void uncheck(item)} />
              ))}
            </ul>
          )}
        </div>
      )}
      <QuickAdd onAdd={quickAdd} />
    </div>
  );
}
```

Run: `npx vitest run src/widget` → Expected: PASS (10 tests)

- [ ] **Step 6: `Setup.tsx` (첫 실행 · 저장 폴더 못 찾음 · 손상 복구)**

```tsx
import { useState } from "react";
import { FolderOpen, Info } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { api, errorMessage, type AppStatus, type TemplateName } from "@/lib/api";
import { cn } from "@/lib/utils";

const TEMPLATES: { value: TemplateName; title: string; desc: string }[] = [
  { value: "homeroom", title: "담임", desc: "출결 확인 · 누가기록 · 알림장 등 6개" },
  { value: "subject", title: "교과전담", desc: "수업 준비 · 진도 체크 등 4개" },
  { value: "empty", title: "비어있음", desc: "직접 하나씩 추가할게요" },
];

export function Setup({ status }: { status: AppStatus }) {
  const [dir, setDir] = useState(status.suggestedDir);
  const [template, setTemplate] = useState<TemplateName>("homeroom");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function pick() {
    const picked = await open({ directory: true, title: "데이터를 저장할 폴더 선택" });
    if (typeof picked === "string") setDir(picked);
  }

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-3.5 p-4">
      <div data-tauri-drag-region>
        <div data-tauri-drag-region className="text-base font-semibold">
          G-routine 시작하기
        </div>
        <p data-tauri-drag-region className="mt-0.5 text-xs text-muted-foreground">
          매일 할 일을 작게 띄워 두고 하나씩 지워 보세요.
        </p>
      </div>

      {status.corrupt && (
        <div className="rounded-lg bg-danger-soft p-2.5 text-xs text-danger">
          데이터 파일이 손상된 것 같아요. 자동 백업으로 되돌릴 수 있어요.
          <Button size="sm" variant="outline" className="mt-2 w-full" disabled={busy} onClick={() => void run(api.restoreBackup)}>
            최근 백업으로 복구
          </Button>
        </div>
      )}
      {!status.corrupt && status.previousDir && (
        <div className="rounded-lg bg-chip-due p-2.5 text-xs leading-5 text-chip-due-ink">
          예전 저장 폴더({status.previousDir})를 찾지 못했어요. 폴더를 다시 고르거나 새로 시작하세요.
        </div>
      )}

      <section className="flex flex-col gap-1.5">
        <span className="text-xs font-medium">저장 위치</span>
        {status.portable ? (
          <div className="rounded-lg bg-muted px-2.5 py-2 text-xs">포터블 모드 · 프로그램 폴더의 data</div>
        ) : (
          <div className="flex items-center gap-1.5">
            <div className="min-w-0 flex-1 truncate rounded-lg bg-muted px-2.5 py-2 font-mono text-[11px]" title={dir}>
              {dir}
            </div>
            <Button size="icon" variant="outline" aria-label="저장 폴더 고르기" onClick={() => void pick()}>
              <FolderOpen />
            </Button>
          </div>
        )}
        <p className="flex gap-1 text-[11px] leading-4 text-muted-foreground">
          <Info className="mt-px size-3 shrink-0" />
          복원 프로그램이 있는 PC는 D드라이브에 저장하세요. 이미 데이터가 있는 폴더를 고르면 그대로 불러와요.
        </p>
      </section>

      <section className="flex flex-col gap-1.5">
        <span className="text-xs font-medium">시작 루틴</span>
        <div role="radiogroup" aria-label="시작 루틴" className="flex flex-col gap-1.5">
          {TEMPLATES.map((t) => (
            <button
              key={t.value}
              type="button"
              role="radio"
              aria-checked={template === t.value}
              onClick={() => setTemplate(t.value)}
              className={cn(
                "rounded-lg border px-3 py-2 text-left transition-colors",
                template === t.value ? "border-soft-3 bg-soft" : "border-border hover:bg-muted",
              )}
            >
              <div className="text-[13px] font-medium">{t.title}</div>
              <div className="text-[11px] text-muted-foreground">{t.desc}</div>
            </button>
          ))}
        </div>
      </section>

      {error && (
        <p role="alert" className="text-xs text-danger">
          {error}
        </p>
      )}
      <Button disabled={busy || !dir} onClick={() => void run(() => api.setup(dir, template))}>
        시작하기
      </Button>
    </div>
  );
}
```

- [ ] **Step 7: `WidgetApp.tsx` 교체**

```tsx
import { api } from "@/lib/api";
import { useAutoResize, useData, useSettings } from "@/lib/hooks";
import { Setup } from "./Setup";
import { TodayPanel } from "./TodayPanel";

export function WidgetApp() {
  const { data: status } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);
  const ref = useAutoResize<HTMLDivElement>();
  const pinned = settings?.alwaysOnTop ?? false;

  return (
    <div ref={ref} className="p-1.5">
      <div className="overflow-hidden rounded-xl border border-border bg-background shadow-[0_2px_10px_rgba(44,44,42,0.10)]">
        {!status ? (
          <div className="h-28" />
        ) : ready ? (
          <TodayPanel pinned={pinned} onTogglePin={() => void update("always_on_top", String(!pinned))} />
        ) : (
          <Setup status={status} />
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 8: 확인과 커밋**

Run: `npm test` → Expected: PASS
Run: `npm run build` → Expected: 성공

```bash
git add -A
git commit -m "feat(widget): today list with check/undo, quick add, links and first-run setup

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: 관리 창 — 루틴 관리 탭

**Files:**
- Create: `src/manager/routines.ts`, `src/manager/routines.test.ts`, `src/manager/RoutineForm.tsx`, `src/manager/RoutineForm.test.tsx`, `src/manager/RoutinesTab.tsx`
- Modify: `src/manager/ManagerApp.tsx` (교체. 기록 · 설정 탭은 Task 10에서 채울 자리 표시)

**Interfaces:**
- Consumes: Task 7의 `api`, `useData`, `useSettings`, UI 컴포넌트
- Produces:
  - `routines.ts`: `WEEKDAY_NAMES: string[]`(월부터 일까지), `weekdayLabel(bits): string`, `repeatLabel(r): string`, `toggleBit(bits, i): number`, `moveItem<T>(list, from, to): T[]`, `isAllowedLink(link): boolean`, `validateInput(input): string | null`, `todayString(d?): string`, `emptyInput(): RoutineInput`, `toInput(r: Routine): RoutineInput`, `normalize(input): RoutineInput`
  - `RoutineForm({ initial?: Routine; onSubmit(input): Promise<void>; onCancel?(): void })`

- [ ] **Step 1: `routines.test.ts` (실패하는 테스트)**

```ts
import type { RoutineInput } from "@/lib/api";
import {
  emptyInput,
  isAllowedLink,
  moveItem,
  normalize,
  repeatLabel,
  todayString,
  toggleBit,
  validateInput,
  weekdayLabel,
} from "./routines";

const base: RoutineInput = { title: "출결 확인", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null };

test("weekday and repeat labels", () => {
  expect(weekdayLabel(5)).toBe("월·수");
  expect(weekdayLabel(16)).toBe("금");
  expect(repeatLabel({ repeatType: "daily", weekdays: 0, onceDate: null })).toBe("매일");
  expect(repeatLabel({ repeatType: "weekdays", weekdays: 17, onceDate: null })).toBe("월·금");
  expect(repeatLabel({ repeatType: "once", weekdays: 0, onceDate: "2026-10-05" })).toBe("10/5 하루");
});

test("bit toggle and list move", () => {
  expect(toggleBit(0, 0)).toBe(1);
  expect(toggleBit(5, 2)).toBe(1);
  expect(moveItem(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
  expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
});

test("link rules mirror the backend", () => {
  expect(isAllowedLink("https://www.neis.go.kr")).toBe(true);
  expect(isAllowedLink("C:\\Program Files\\app.exe")).toBe(true);
  expect(isAllowedLink("\\\\server\\share")).toBe(true);
  expect(isAllowedLink("https://")).toBe(false);
  expect(isAllowedLink("javascript:alert(1)")).toBe(false);
  expect(isAllowedLink("notepad.exe")).toBe(false);
});

test("validation messages", () => {
  expect(validateInput(base)).toBeNull();
  expect(validateInput({ ...base, title: "  " })).toBe("루틴 이름을 입력해 주세요");
  expect(validateInput({ ...base, title: "가".repeat(41) })).toBe("루틴 이름은 40자 이내로 입력해 주세요");
  expect(validateInput({ ...base, repeatType: "weekdays", weekdays: 0 })).toBe("요일을 하나 이상 골라 주세요");
  expect(validateInput({ ...base, repeatType: "once", onceDate: null })).toBe("날짜를 골라 주세요");
  expect(validateInput({ ...base, link: "ftp://x" })).toBe("바로가기는 https:// 주소나 C:\\ 같은 프로그램 경로만 넣을 수 있어요");
});

test("normalize trims and clears fields that do not apply", () => {
  expect(normalize({ ...base, title: " 수업 준비 ", weekdays: 3, link: "  ", dueTime: "" })).toEqual({
    ...base,
    title: "수업 준비",
    weekdays: 0,
    link: null,
    dueTime: null,
  });
  expect(normalize({ ...base, repeatType: "once", onceDate: null }).onceDate).toBe(todayString());
  expect(emptyInput()).toEqual({ title: "", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null });
});
```

Run: `npx vitest run src/manager/routines.test.ts` → Expected: FAIL

- [ ] **Step 2: `routines.ts` 구현**

```ts
import type { RepeatType, Routine, RoutineInput } from "@/lib/api";

export const WEEKDAY_NAMES = ["월", "화", "수", "목", "금", "토", "일"];
export const REPEAT_NAMES: Record<RepeatType, string> = { daily: "매일", weekdays: "요일 지정", once: "하루만" };

const pad = (n: number) => String(n).padStart(2, "0");

export function weekdayLabel(bits: number): string {
  return WEEKDAY_NAMES.filter((_, i) => (bits & (1 << i)) !== 0).join("·");
}

export function repeatLabel(r: Pick<Routine, "repeatType" | "weekdays" | "onceDate">): string {
  if (r.repeatType === "daily") return "매일";
  if (r.repeatType === "weekdays") return weekdayLabel(r.weekdays);
  if (!r.onceDate) return "하루";
  return `${Number(r.onceDate.slice(5, 7))}/${Number(r.onceDate.slice(8, 10))} 하루`;
}

export function toggleBit(bits: number, i: number): number {
  return bits ^ (1 << i);
}

export function moveItem<T>(list: T[], from: number, to: number): T[] {
  const next = [...list];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

export function isAllowedLink(link: string): boolean {
  const lower = link.toLowerCase();
  for (const scheme of ["https://", "http://"]) {
    if (lower.startsWith(scheme)) return link.length > scheme.length;
  }
  return /^[a-zA-Z]:[\\/]/.test(link) || link.startsWith("\\\\");
}

export function todayString(d: Date = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function emptyInput(): RoutineInput {
  return { title: "", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null };
}

export function toInput(r: Routine): RoutineInput {
  return { title: r.title, repeatType: r.repeatType, weekdays: r.weekdays, onceDate: r.onceDate, dueTime: r.dueTime, link: r.link };
}

/** 저장 전 정리: 공백 제거, 반복 종류에 맞지 않는 값 비우기 */
export function normalize(input: RoutineInput): RoutineInput {
  return {
    title: input.title.trim(),
    repeatType: input.repeatType,
    weekdays: input.repeatType === "weekdays" ? input.weekdays : 0,
    onceDate: input.repeatType === "once" ? (input.onceDate ?? todayString()) : null,
    dueTime: input.dueTime ? input.dueTime : null,
    link: input.link?.trim() ? input.link.trim() : null,
  };
}

export function validateInput(input: RoutineInput): string | null {
  const title = input.title.trim();
  if (!title) return "루틴 이름을 입력해 주세요";
  if ([...title].length > 40) return "루틴 이름은 40자 이내로 입력해 주세요";
  if (input.repeatType === "weekdays" && (input.weekdays & 0x7f) === 0) return "요일을 하나 이상 골라 주세요";
  if (input.repeatType === "once" && !input.onceDate) return "날짜를 골라 주세요";
  if (input.link && input.link.trim() && !isAllowedLink(input.link.trim())) {
    return "바로가기는 https:// 주소나 C:\\ 같은 프로그램 경로만 넣을 수 있어요";
  }
  return null;
}
```

> 위젯 칩은 "오늘만", 관리 탭 반복 버튼은 "하루만"으로 표기한다. 관리 탭에서는 오늘이 아닌 날짜도 고를 수 있기 때문이다.

Run: `npx vitest run src/manager/routines.test.ts` → Expected: PASS (5 tests)

- [ ] **Step 3: `RoutineForm.test.tsx` (실패하는 테스트)**

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RoutineForm } from "./RoutineForm";

test("shows validation error and does not submit empty title", async () => {
  const onSubmit = vi.fn();
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(screen.getByRole("alert")).toHaveTextContent("루틴 이름을 입력해 주세요");
  expect(onSubmit).not.toHaveBeenCalled();
});

test("requires a weekday when repeat is weekdays", async () => {
  const onSubmit = vi.fn();
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.type(screen.getByLabelText("루틴 이름"), "주간학습안내");
  await user.click(screen.getByRole("radio", { name: "요일 지정" }));
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(screen.getByRole("alert")).toHaveTextContent("요일을 하나 이상 골라 주세요");
});

test("submits normalized input and resets", async () => {
  const onSubmit = vi.fn().mockResolvedValue(undefined);
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.type(screen.getByLabelText("루틴 이름"), " 급식 지도 ");
  await user.click(screen.getByRole("radio", { name: "요일 지정" }));
  await user.click(screen.getByRole("button", { name: "월" }));
  await user.click(screen.getByRole("button", { name: "수" }));
  await user.type(screen.getByLabelText("바로가기"), "https://a.b");
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(onSubmit).toHaveBeenCalledWith({
    title: "급식 지도",
    repeatType: "weekdays",
    weekdays: 5,
    onceDate: null,
    dueTime: null,
    link: "https://a.b",
  });
  expect(screen.getByLabelText("루틴 이름")).toHaveValue("");
});

test("edit mode shows existing values and a save button", () => {
  render(
    <RoutineForm
      initial={{
        id: 1,
        title: "출결 확인",
        repeatType: "daily",
        weekdays: 0,
        onceDate: null,
        dueTime: "09:00",
        link: null,
        sortOrder: 0,
        createdAt: "",
        archivedAt: null,
      }}
      onSubmit={vi.fn()}
      onCancel={vi.fn()}
    />,
  );
  expect(screen.getByLabelText("루틴 이름")).toHaveValue("출결 확인");
  expect(screen.getByLabelText("마감 시각")).toHaveValue("09:00");
  expect(screen.getByRole("button", { name: "저장" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "취소" })).toBeInTheDocument();
});
```

Run: `npx vitest run src/manager/RoutineForm.test.tsx` → Expected: FAIL

- [ ] **Step 4: `RoutineForm.tsx` 구현**

```tsx
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { errorMessage, type RepeatType, type Routine, type RoutineInput } from "@/lib/api";
import { cn } from "@/lib/utils";
import { emptyInput, normalize, REPEAT_NAMES, todayString, toggleBit, toInput, validateInput, WEEKDAY_NAMES } from "./routines";

interface Props {
  initial?: Routine;
  onSubmit: (input: RoutineInput) => Promise<void>;
  onCancel?: () => void;
}

const REPEATS: RepeatType[] = ["daily", "weekdays", "once"];

export function RoutineForm({ initial, onSubmit, onCancel }: Props) {
  const [input, setInput] = useState<RoutineInput>(() => (initial ? toInput(initial) : emptyInput()));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const set = (patch: Partial<RoutineInput>) => {
    setInput((i) => ({ ...i, ...patch }));
    setError(null);
  };

  async function submit() {
    const normalized = normalize(input);
    const message = validateInput(normalized);
    if (message) {
      setError(message);
      return;
    }
    setBusy(true);
    try {
      await onSubmit(normalized);
      if (!initial) setInput(emptyInput());
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-2.5 rounded-xl bg-soft/50 p-3.5">
      <Input
        aria-label="루틴 이름"
        placeholder="루틴 이름 (예: 급식 지도)"
        value={input.title}
        maxLength={40}
        onChange={(e) => set({ title: e.target.value })}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing) {
            e.preventDefault();
            void submit();
          }
        }}
      />
      <div role="radiogroup" aria-label="반복" className="flex overflow-hidden rounded-lg border border-input bg-background text-xs">
        {REPEATS.map((t) => (
          <button
            key={t}
            type="button"
            role="radio"
            aria-checked={input.repeatType === t}
            onClick={() => set({ repeatType: t, onceDate: t === "once" ? (input.onceDate ?? todayString()) : input.onceDate })}
            className={cn("flex-1 py-1.5", input.repeatType === t ? "bg-soft font-medium text-ink" : "text-muted-foreground hover:bg-muted")}
          >
            {REPEAT_NAMES[t]}
          </button>
        ))}
      </div>
      {input.repeatType === "weekdays" && (
        <div className="flex gap-1">
          {WEEKDAY_NAMES.slice(0, 5).map((name, i) => {
            const on = (input.weekdays & (1 << i)) !== 0;
            return (
              <button
                key={name}
                type="button"
                aria-pressed={on}
                onClick={() => set({ weekdays: toggleBit(input.weekdays, i) })}
                className={cn(
                  "flex-1 rounded-md py-1 text-xs",
                  on ? "bg-soft-3 text-ink" : "border border-input bg-background text-muted-foreground",
                )}
              >
                {name}
              </button>
            );
          })}
        </div>
      )}
      {input.repeatType === "once" && (
        <Input type="date" aria-label="날짜" value={input.onceDate ?? ""} onChange={(e) => set({ onceDate: e.target.value || null })} />
      )}
      <div className="flex gap-2">
        <Input
          type="time"
          aria-label="마감 시각"
          className="w-32 shrink-0"
          value={input.dueTime ?? ""}
          onChange={(e) => set({ dueTime: e.target.value || null })}
        />
        <Input
          aria-label="바로가기"
          placeholder="바로가기: https://… 또는 C:\…\프로그램.exe"
          value={input.link ?? ""}
          onChange={(e) => set({ link: e.target.value })}
        />
      </div>
      {error && (
        <p role="alert" className="text-xs text-danger">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-2">
        {onCancel && (
          <Button variant="ghost" size="sm" onClick={onCancel}>
            취소
          </Button>
        )}
        <Button size="sm" disabled={busy} onClick={() => void submit()}>
          {initial ? "저장" : "추가"}
        </Button>
      </div>
    </div>
  );
}
```

Run: `npx vitest run src/manager` → Expected: PASS (9 tests)

- [ ] **Step 5: `RoutinesTab.tsx`**

드래그는 HTML5 드래그 앤 드롭으로 구현한다. manager 창은 `disable_drag_drop_handler()`로 만들어서 브라우저 DnD가 동작한다.

```tsx
import { useState } from "react";
import { GripVertical, Link as LinkIcon, Pencil, Trash2 } from "lucide-react";
import { Chip } from "@/components/Chip";
import { Button } from "@/components/ui/button";
import { api, errorMessage } from "@/lib/api";
import { useData } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { RoutineForm } from "./RoutineForm";
import { moveItem, repeatLabel } from "./routines";

export function RoutinesTab() {
  const { data, setData } = useData(api.listRoutines);
  const [editing, setEditing] = useState<number | null>(null);
  const [confirming, setConfirming] = useState<number | null>(null);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const routines = data ?? [];

  async function drop(to: number) {
    if (dragFrom === null || dragFrom === to) {
      setDragFrom(null);
      return;
    }
    const next = moveItem(routines, dragFrom, to);
    setData(next);
    setDragFrom(null);
    try {
      await api.reorderRoutines(next.map((r) => r.id));
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }

  async function remove(id: number) {
    try {
      await api.archiveRoutine(id);
    } catch (e) {
      setNotice(errorMessage(e));
    }
    setConfirming(null);
  }

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_300px] gap-6">
      <section>
        <h2 className="mb-2 text-sm font-semibold">
          내 루틴 <span className="font-normal text-muted-foreground">{routines.length}개</span>
        </h2>
        {routines.length === 0 ? (
          <p className="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
            아직 루틴이 없어요. 오른쪽에서 첫 루틴을 추가해 보세요.
          </p>
        ) : (
          <ul className="divide-y divide-border rounded-xl border border-border">
            {routines.map((r, idx) =>
              editing === r.id ? (
                <li key={r.id} className="p-2">
                  <RoutineForm
                    initial={r}
                    onSubmit={async (input) => {
                      await api.updateRoutine(r.id, input);
                      setEditing(null);
                    }}
                    onCancel={() => setEditing(null)}
                  />
                </li>
              ) : (
                <li
                  key={r.id}
                  draggable
                  onDragStart={() => setDragFrom(idx)}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => void drop(idx)}
                  onDragEnd={() => setDragFrom(null)}
                  className={cn("flex items-center gap-2 px-3 py-2.5 text-sm", dragFrom === idx && "opacity-50")}
                >
                  <GripVertical className="size-4 shrink-0 cursor-grab text-muted-foreground" aria-hidden />
                  <span className="min-w-0 flex-1 truncate">{r.title}</span>
                  <Chip kind={r.repeatType}>{repeatLabel(r)}</Chip>
                  {r.dueTime && <Chip kind="due">{r.dueTime}</Chip>}
                  {r.link && <LinkIcon className="size-3.5 shrink-0 text-primary" aria-label="바로가기 있음" />}
                  {confirming === r.id ? (
                    <span className="flex shrink-0 items-center gap-1 text-xs">
                      <span className="text-muted-foreground">삭제할까요? 지난 기록은 남아요</span>
                      <Button size="sm" variant="destructive" onClick={() => void remove(r.id)}>
                        삭제
                      </Button>
                      <Button size="sm" variant="ghost" onClick={() => setConfirming(null)}>
                        취소
                      </Button>
                    </span>
                  ) : (
                    <>
                      <Button variant="ghost" size="icon-sm" aria-label={`${r.title} 수정`} onClick={() => setEditing(r.id)}>
                        <Pencil className="size-3.5" />
                      </Button>
                      <Button variant="ghost" size="icon-sm" aria-label={`${r.title} 삭제`} onClick={() => setConfirming(r.id)}>
                        <Trash2 className="size-3.5" />
                      </Button>
                    </>
                  )}
                </li>
              ),
            )}
          </ul>
        )}
        {notice && <p className="mt-2 text-xs text-danger">{notice}</p>}
        <p className="mt-2 text-[11px] text-muted-foreground">줄을 끌어서 순서를 바꿀 수 있어요.</p>
      </section>
      <section>
        <h2 className="mb-2 text-sm font-semibold">새 루틴</h2>
        <RoutineForm onSubmit={async (input) => void (await api.createRoutine(input))} />
      </section>
    </div>
  );
}
```

`Chip kind={r.repeatType}`: `RepeatType`(`daily | weekdays | once`)는 `ChipKind`의 부분집합이라 그대로 넘길 수 있다.

- [ ] **Step 6: `ManagerApp.tsx` 교체**

```tsx
import { api } from "@/lib/api";
import { useData, useSettings } from "@/lib/hooks";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { RoutinesTab } from "./RoutinesTab";

export function ManagerApp() {
  const { data: status } = useData(api.status);
  const ready = status?.ready ?? false;
  useSettings(ready);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="flex items-center gap-2.5 border-b border-border px-6 py-4">
        <div className="size-6 rounded-lg bg-soft-3" />
        <h1 className="text-base font-semibold">G-routine 관리</h1>
      </header>
      {status && !ready && <p className="p-6 text-sm text-muted-foreground">먼저 위젯에서 시작 설정을 마쳐 주세요.</p>}
      {ready && (
        <Tabs defaultValue="routines" className="px-6 py-4">
          <TabsList>
            <TabsTrigger value="routines">루틴 관리</TabsTrigger>
            <TabsTrigger value="history">완료 기록</TabsTrigger>
            <TabsTrigger value="settings">설정</TabsTrigger>
          </TabsList>
          <TabsContent value="routines">
            <RoutinesTab />
          </TabsContent>
          <TabsContent value="history">
            <p className="text-sm text-muted-foreground">준비 중</p>
          </TabsContent>
          <TabsContent value="settings">
            <p className="text-sm text-muted-foreground">준비 중</p>
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}
```

- [ ] **Step 7: 확인과 커밋**

Run: `npm test && npm run build` → Expected: 모두 성공

```bash
git add -A
git commit -m "feat(manager): routine list with add/edit/delete and drag reorder

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: 관리 창 — 완료 기록 탭, 설정 탭

**Files:**
- Create: `src/manager/calendar.ts`, `src/manager/calendar.test.ts`, `src/manager/HistoryTab.tsx`, `src/manager/SettingsTab.tsx`
- Modify: `src/manager/ManagerApp.tsx` (두 탭 연결)

**Interfaces:**
- Consumes: `api.historyMonth/historyDay/status/exportBackup/importBackup/changeDataDir`, `useData`, `useSettings().update`, `formatTime`(Task 8 `widget/today.ts`), `todayString`(Task 9)
- Produces: `calendar.ts`: `buildMonthGrid(year, month): (string | null)[]`, `levelOf(s?: DaySummary): -1 | 0 | 1 | 2 | 3`, `shiftMonth(year, month, delta): { year: number; month: number }`, `formatDayTitle(day): string`, `formatHour(h): string`

- [ ] **Step 1: `calendar.test.ts` (실패하는 테스트)**

```ts
import { buildMonthGrid, formatDayTitle, formatHour, levelOf, shiftMonth } from "./calendar";

test("month grid starts on Sunday with leading blanks", () => {
  const cells = buildMonthGrid(2026, 10); // 2026-10-01은 목요일
  expect(cells.slice(0, 5)).toEqual([null, null, null, null, "2026-10-01"]);
  expect(cells).toHaveLength(35);
  expect(cells[cells.length - 1]).toBe("2026-10-31");
});

test("completion level buckets", () => {
  expect(levelOf(undefined)).toBe(-1);
  expect(levelOf({ day: "d", total: 0, completed: 0 })).toBe(-1);
  expect(levelOf({ day: "d", total: 6, completed: 0 })).toBe(0);
  expect(levelOf({ day: "d", total: 6, completed: 2 })).toBe(1);
  expect(levelOf({ day: "d", total: 6, completed: 3 })).toBe(2);
  expect(levelOf({ day: "d", total: 6, completed: 6 })).toBe(3);
});

test("month shifting wraps years", () => {
  expect(shiftMonth(2026, 12, 1)).toEqual({ year: 2027, month: 1 });
  expect(shiftMonth(2026, 1, -1)).toEqual({ year: 2025, month: 12 });
  expect(shiftMonth(2026, 10, 1)).toEqual({ year: 2026, month: 11 });
});

test("labels", () => {
  expect(formatDayTitle("2026-10-02")).toBe("10월 2일 (금)");
  expect(formatHour(0)).toBe("자정 (0시)");
  expect(formatHour(4)).toBe("오전 4시");
  expect(formatHour(12)).toBe("낮 12시");
});
```

Run: `npx vitest run src/manager/calendar.test.ts` → Expected: FAIL

- [ ] **Step 2: `calendar.ts` 구현**

```ts
import type { DaySummary } from "@/lib/api";

const pad = (n: number) => String(n).padStart(2, "0");
const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];

export type Level = -1 | 0 | 1 | 2 | 3;

export function buildMonthGrid(year: number, month: number): (string | null)[] {
  const first = new Date(year, month - 1, 1).getDay();
  const days = new Date(year, month, 0).getDate();
  const cells: (string | null)[] = Array.from({ length: first }, () => null);
  for (let d = 1; d <= days; d++) cells.push(`${year}-${pad(month)}-${pad(d)}`);
  return cells;
}

export function levelOf(s?: DaySummary): Level {
  if (!s || s.total === 0) return -1;
  if (s.completed === 0) return 0;
  if (s.completed >= s.total) return 3;
  return s.completed * 2 >= s.total ? 2 : 1;
}

export function shiftMonth(year: number, month: number, delta: number) {
  const index = year * 12 + (month - 1) + delta;
  return { year: Math.floor(index / 12), month: (index % 12) + 1 };
}

export function formatDayTitle(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  return `${m}월 ${d}일 (${WEEKDAYS[new Date(y, m - 1, d).getDay()]})`;
}

export function formatHour(h: number): string {
  if (h === 0) return "자정 (0시)";
  if (h === 12) return "낮 12시";
  return h < 12 ? `오전 ${h}시` : `오후 ${h - 12}시`;
}
```

Run: `npx vitest run src/manager/calendar.test.ts` → Expected: PASS (4 tests)

- [ ] **Step 3: `HistoryTab.tsx`**

```tsx
import { useState } from "react";
import { Check, ChevronLeft, ChevronRight, Circle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { useData } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { formatTime } from "@/widget/today";
import { buildMonthGrid, formatDayTitle, levelOf, shiftMonth, type Level } from "./calendar";
import { todayString } from "./routines";

const LEVEL_CLASS: Record<Level, string> = {
  [-1]: "text-muted-foreground/60",
  0: "bg-muted text-muted-foreground",
  1: "bg-soft text-ink",
  2: "bg-soft-2 text-ink",
  3: "bg-soft-3 text-ink font-medium",
};

export function HistoryTab() {
  const now = new Date();
  const [ym, setYm] = useState({ year: now.getFullYear(), month: now.getMonth() + 1 });
  const [selected, setSelected] = useState(todayString());
  const { data: summary } = useData(() => api.historyMonth(ym.year, ym.month), [ym.year, ym.month]);
  const { data: items } = useData(() => api.historyDay(selected), [selected]);

  const byDay = new Map((summary ?? []).map((s) => [s.day, s]));
  const cells = buildMonthGrid(ym.year, ym.month);
  const list = items ?? [];
  const doneCount = list.filter((i) => i.completedAt).length;

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_280px] gap-6">
      <section>
        <div className="mb-3 flex items-center justify-between">
          <Button variant="ghost" size="icon" aria-label="이전 달" onClick={() => setYm(shiftMonth(ym.year, ym.month, -1))}>
            <ChevronLeft />
          </Button>
          <span className="text-sm font-semibold">
            {ym.year}년 {ym.month}월
          </span>
          <Button variant="ghost" size="icon" aria-label="다음 달" onClick={() => setYm(shiftMonth(ym.year, ym.month, 1))}>
            <ChevronRight />
          </Button>
        </div>
        <div className="grid grid-cols-7 gap-1 text-center text-xs">
          {["일", "월", "화", "수", "목", "금", "토"].map((d) => (
            <div key={d} className="py-1 text-muted-foreground">
              {d}
            </div>
          ))}
          {cells.map((day, i) =>
            day ? (
              <button
                key={day}
                type="button"
                aria-label={formatDayTitle(day)}
                aria-pressed={selected === day}
                onClick={() => setSelected(day)}
                className={cn(
                  "aspect-square rounded-lg py-1 transition-colors hover:ring-1 hover:ring-soft-3",
                  LEVEL_CLASS[levelOf(byDay.get(day))],
                  selected === day && "ring-2 ring-strong",
                )}
              >
                {Number(day.slice(8))}
              </button>
            ) : (
              <div key={`blank-${i}`} />
            ),
          )}
        </div>
        <div className="mt-3 flex items-center justify-end gap-1.5 text-[11px] text-muted-foreground">
          적음
          <span className="size-3 rounded bg-muted" />
          <span className="size-3 rounded bg-soft" />
          <span className="size-3 rounded bg-soft-2" />
          <span className="size-3 rounded bg-soft-3" />
          모두 완료
        </div>
      </section>
      <section className="rounded-xl border border-border p-4">
        <h2 className="text-sm font-semibold">{formatDayTitle(selected)}</h2>
        {list.length === 0 ? (
          <p className="mt-3 text-xs text-muted-foreground">이날은 기록이 없어요.</p>
        ) : (
          <>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {list.length}개 중 {doneCount}개 완료
            </p>
            <ul className="mt-3 flex flex-col gap-1.5 text-[13px]">
              {list.map((item) => (
                <li key={item.id} className="flex items-center gap-2">
                  {item.completedAt ? (
                    <Check className="size-4 shrink-0 text-success" />
                  ) : (
                    <Circle className="size-4 shrink-0 text-input" />
                  )}
                  <span className={cn("min-w-0 flex-1 truncate", !item.completedAt && "text-muted-foreground")}>{item.title}</span>
                  <span className="text-[11px] text-muted-foreground">{item.completedAt ? formatTime(item.completedAt) : "미완료"}</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>
    </div>
  );
}
```

- [ ] **Step 4: `SettingsTab.tsx`**

```tsx
import { useState, type ReactNode } from "react";
import { Download, FolderOpen, Info, Upload } from "lucide-react";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { api, errorMessage, type AppStatus, type SettingKey, type Settings, type ThemeName } from "@/lib/api";
import { cn } from "@/lib/utils";
import { formatHour } from "./calendar";
import { todayString } from "./routines";

const THEMES: { value: ThemeName; label: string; color: string }[] = [
  { value: "lavender", label: "라벤더", color: "#AFA9EC" },
  { value: "mint", label: "민트", color: "#9FE1CB" },
  { value: "peach", label: "피치", color: "#F5C4B3" },
  { value: "sky", label: "스카이", color: "#B5D4F4" },
  { value: "lemon", label: "레몬", color: "#FAC775" },
];
const JSON_FILTER = [{ name: "G-routine 백업", extensions: ["json"] }];

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-border py-3">
      <div>
        <div className="text-sm">{label}</div>
        {hint && <div className="text-xs text-muted-foreground">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

interface Props {
  settings: Settings;
  status: AppStatus;
  onChange: (key: SettingKey, value: string) => Promise<Settings>;
}

export function SettingsTab({ settings, status, onChange }: Props) {
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);

  async function run(action: () => Promise<unknown>, ok?: string) {
    try {
      await action();
      setMessage(ok ? { ok: true, text: ok } : null);
    } catch (e) {
      setMessage({ ok: false, text: errorMessage(e) });
    }
  }

  const toggle = (key: SettingKey) => (checked: boolean) => void run(() => onChange(key, String(checked)));

  async function changeDir() {
    const picked = await open({ directory: true, title: "새 저장 폴더 선택" });
    if (typeof picked === "string") await run(() => api.changeDataDir(picked), "저장 위치를 바꿨어요");
  }

  async function exportBackup() {
    const path = await save({ defaultPath: `g-routine-backup-${todayString()}.json`, filters: JSON_FILTER });
    if (path) await run(() => api.exportBackup(path), "백업 파일을 저장했어요");
  }

  async function importBackup() {
    const path = await open({ multiple: false, directory: false, filters: JSON_FILTER });
    if (typeof path !== "string") return;
    const ok = await confirm("지금 데이터를 백업 파일 내용으로 바꿀까요? 되돌릴 수 없어요.", {
      title: "백업 불러오기",
      kind: "warning",
    });
    if (ok) await run(() => api.importBackup(path), "백업을 불러왔어요");
  }

  return (
    <div className="max-w-xl">
      <Row label="항상 맨 위에 표시" hint="다른 창에 가려지지 않아요">
        <Switch aria-label="항상 맨 위에 표시" checked={settings.alwaysOnTop} onCheckedChange={toggle("always_on_top")} />
      </Row>
      <Row label="컴퓨터 켜면 자동 시작">
        <Switch aria-label="컴퓨터 켜면 자동 시작" checked={settings.autostart} onCheckedChange={toggle("autostart")} />
      </Row>
      <Row label="주말에는 숨기기" hint="토 · 일에는 반복 루틴을 띄우지 않아요">
        <Switch aria-label="주말에는 숨기기" checked={settings.hideWeekends} onCheckedChange={toggle("hide_weekends")} />
      </Row>
      <Row label="하루 시작 시각" hint="이 시각 전에 체크하면 전날 기록으로 남아요">
        <select
          aria-label="하루 시작 시각"
          value={settings.dayStartHour}
          onChange={(e) => void run(() => onChange("day_start_hour", e.target.value))}
          className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
        >
          {Array.from({ length: 13 }, (_, h) => (
            <option key={h} value={h}>
              {formatHour(h)}
            </option>
          ))}
        </select>
      </Row>
      <Row label="테마색">
        <div className="flex gap-2">
          {THEMES.map((t) => (
            <button
              key={t.value}
              type="button"
              aria-label={t.label}
              aria-pressed={settings.theme === t.value}
              onClick={() => void run(() => onChange("theme", t.value))}
              className={cn("size-6 rounded-full", settings.theme === t.value && "ring-2 ring-strong ring-offset-2")}
              style={{ background: t.color }}
            />
          ))}
        </div>
      </Row>
      <div className="border-b border-border py-3">
        <div className="mb-1.5 text-sm">데이터 저장 위치</div>
        <div className="flex items-center gap-2">
          <div className="min-w-0 flex-1 truncate rounded-lg bg-muted px-3 py-2 font-mono text-xs" title={status.dataDir ?? ""}>
            {status.portable ? "포터블 모드 · 프로그램 폴더의 data" : status.dataDir}
          </div>
          {!status.portable && (
            <Button variant="outline" size="sm" onClick={() => void changeDir()}>
              <FolderOpen />
              변경
            </Button>
          )}
        </div>
        <p className="mt-1.5 flex gap-1 text-xs text-chip-due-ink">
          <Info className="mt-0.5 size-3 shrink-0" />
          복원 프로그램이 있는 PC는 D드라이브 같은 보존되는 위치에 저장하세요.
        </p>
      </div>
      <div className="flex gap-2 py-4">
        <Button variant="outline" size="sm" onClick={() => void exportBackup()}>
          <Download />
          백업 내보내기
        </Button>
        <Button variant="outline" size="sm" onClick={() => void importBackup()}>
          <Upload />
          불러오기
        </Button>
      </div>
      {message && (
        <p role={message.ok ? "status" : "alert"} className={cn("text-xs", message.ok ? "text-strong" : "text-danger")}>
          {message.text}
        </p>
      )}
    </div>
  );
}
```

- [ ] **Step 5: `ManagerApp.tsx`에 두 탭 연결**

`ManagerApp`을 다음으로 교체한다.

```tsx
import { api } from "@/lib/api";
import { useData, useSettings } from "@/lib/hooks";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { HistoryTab } from "./HistoryTab";
import { RoutinesTab } from "./RoutinesTab";
import { SettingsTab } from "./SettingsTab";

export function ManagerApp() {
  const { data: status } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="flex items-center gap-2.5 border-b border-border px-6 py-4">
        <div className="size-6 rounded-lg bg-soft-3" />
        <h1 className="text-base font-semibold">G-routine 관리</h1>
      </header>
      {status && !ready && <p className="p-6 text-sm text-muted-foreground">먼저 위젯에서 시작 설정을 마쳐 주세요.</p>}
      {status && ready && (
        <Tabs defaultValue="routines" className="px-6 py-4">
          <TabsList>
            <TabsTrigger value="routines">루틴 관리</TabsTrigger>
            <TabsTrigger value="history">완료 기록</TabsTrigger>
            <TabsTrigger value="settings">설정</TabsTrigger>
          </TabsList>
          <TabsContent value="routines">
            <RoutinesTab />
          </TabsContent>
          <TabsContent value="history">
            <HistoryTab />
          </TabsContent>
          <TabsContent value="settings">
            {settings && <SettingsTab settings={settings} status={status} onChange={update} />}
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}
```

- [ ] **Step 6: 확인과 커밋**

Run: `npm test && npm run build` → Expected: 모두 성공

```bash
git add -A
git commit -m "feat(manager): history calendar and settings tab

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: 빌드 · 포터블 패키지 · 문서 · 검증

**Files:**
- Create: `scripts/package-portable.ps1`, `README.md`, `docs/manual-checklist.md`
- Modify: `.gitignore` (`release/` 추가)

**Interfaces:**
- Consumes: 완성된 앱 전체
- Produces: `src-tauri/target/release/bundle/nsis/G-routine_0.1.0_x64-setup.exe`, `release/G-routine_0.1.0_portable.zip`

- [ ] **Step 1: 전체 테스트**

Run: `cd src-tauri && cargo test` → Expected: 전부 PASS
Run: `cd .. && npm test` → Expected: 전부 PASS

- [ ] **Step 2: 릴리스 빌드**

Run: `npm run tauri build` (5~10분 걸릴 수 있음, 시간 제한을 넉넉히 둔다) → Expected: `src-tauri/target/release/bundle/nsis/` 아래 `G-routine_0.1.0_x64-setup.exe` 생성

- [ ] **Step 3: `scripts/package-portable.ps1`**

```powershell
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$version = (Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json).version
$exe = Join-Path $root "src-tauri/target/release/g-routine.exe"
if (-not (Test-Path $exe)) { throw "먼저 'npm run tauri build'를 실행하세요." }

$out = Join-Path $root "release"
$stage = Join-Path $out "G-routine-portable"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage "data") | Out-Null
Copy-Item $exe (Join-Path $stage "G-routine.exe")
Set-Content -Path (Join-Path $stage "data/README.txt") -Encoding utf8 -Value "이 폴더에 G-routine 데이터가 저장됩니다. 프로그램과 함께 옮기세요."

$zip = Join-Path $out "G-routine_${version}_portable.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip

$installer = Get-ChildItem (Join-Path $root "src-tauri/target/release/bundle/nsis") -Filter "*.exe" | Select-Object -First 1
if ($installer) { Copy-Item $installer.FullName $out }
Get-ChildItem $out -File | ForEach-Object { "{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB) }
```

`.gitignore`에 `release/`를 추가한다.

Run: `powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1` → Expected: `release/`에 설치 파일과 포터블 zip, 크기 출력. 설치 파일은 15MB 이하여야 한다.

- [ ] **Step 4: 실행 스모크 테스트 (포터블 모드, 사용자 PC 설정을 건드리지 않음)**

PowerShell:

```powershell
$tmp = Join-Path $env:TEMP "g-routine-smoke"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
Expand-Archive release/G-routine_0.1.0_portable.zip $tmp
$p = Start-Process (Join-Path $tmp "G-routine.exe") -PassThru
Start-Sleep -Seconds 6
$alive = -not $p.HasExited
$mem = if ($alive) { [math]::Round((Get-Process -Id $p.Id).WorkingSet64 / 1MB, 1) } else { 0 }
"alive=$alive memMB=$mem"
if ($alive) { Stop-Process -Id $p.Id -Force }
Get-Process msedgewebview2 -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.StartTime -gt (Get-Date).AddMinutes(-2) } | Measure-Object | Select-Object -ExpandProperty Count
```

Expected: `alive=True`. 포터블 모드라 `%AppData%` 위치 파일과 레지스트리 자동 시작을 쓰지 않는다(첫 실행 설정 전이므로). 메모리 값은 보고서에 기록한다(WebView2 자식 프로세스 메모리는 별도).

- [ ] **Step 5: `docs/manual-checklist.md`**

```markdown
# G-routine 수동 점검표

릴리스 전에 실제 Windows PC에서 확인한다. 통과하면 [x]로 표시한다.

## 설치 · 첫 실행
- [ ] 설치 파일이 관리자 권한 요청 없이 설치된다
- [ ] 첫 실행 시 우측 하단에 "G-routine 시작하기"가 뜬다
- [ ] D드라이브가 있으면 저장 위치 기본값이 `D:\G-routine\data`다
- [ ] 템플릿(담임/교과전담/비어있음)대로 루틴이 생긴다
- [ ] 포터블 zip을 풀고 실행하면 "포터블 모드"로 표시되고 `data\`에 저장된다

## 위젯
- [ ] 작업표시줄 바로 위 우측 하단에 붙는다 (배율 100% / 125% / 150%)
- [ ] 체크하면 부드럽게 사라지고 진행률이 오른다
- [ ] "되돌리기"를 누르면 3초 안에 복구된다
- [ ] "완료 n개 보기"에서 체크 시각이 보이고 해제할 수 있다
- [ ] ↗ 버튼으로 NEIS 등 링크가 열린다
- [ ] 빠른 추가에 한글 입력 후 Enter를 누르면 글자가 깨지거나 중복되지 않는다
- [ ] 헤더를 끌어 옮기면 재실행 후에도 그 위치(아래 모서리 기준)를 기억한다
- [ ] 모니터 구성이 바뀌어 위치가 화면 밖이면 우측 하단으로 돌아온다
- [ ] 핀 버튼으로 맨 위 고정을 켜고 끌 수 있다
- [ ] – 버튼으로 숨기고 트레이 아이콘 클릭으로 다시 보인다

## 관리 창
- [ ] 루틴 추가(매일/요일/하루만, 마감 시각, 바로가기), 수정, 삭제, 드래그 순서 변경
- [ ] 삭제해도 지난 기록에는 그대로 보인다
- [ ] 완료 기록 달력의 농도가 달성률과 맞고, 날짜를 누르면 완료 시각과 미완료 항목이 보인다
- [ ] 설정: 맨 위 고정 · 자동 시작 · 주말 숨기기 · 하루 시작 시각 · 테마색이 바로 반영된다
- [ ] 백업 내보내기 → 불러오기로 같은 데이터가 복원된다
- [ ] 저장 위치 변경 후 재실행해도 데이터가 유지된다

## 날짜 · 복원 프로그램
- [ ] 오전 4시 이전 체크는 전날 기록으로 남는다
- [ ] 자정 / 오전 4시를 지나 켜 둔 채로 두면 1분 안에 새 목록으로 바뀐다
- [ ] 절전에서 깨어나면 목록이 새 날짜로 바뀐다
- [ ] "오늘만" 할 일을 안 하고 다음 날이 되면 목록에서 사라지고 기록에 "미완료"로 남는다
- [ ] `%AppData%\com.groutine.app` 폴더를 지운 뒤 실행해도 `D:\G-routine\data`를 자동으로 다시 찾는다
- [ ] 자동 시작을 켜고 재부팅하면 위젯이 뜬다
- [ ] 이미 실행 중일 때 한 번 더 실행하면 새 창 없이 기존 위젯이 앞으로 나온다
```

- [ ] **Step 6: `README.md`**

````markdown
# G-routine

교사가 매일 해야 할 루틴(출결 확인, 수업 준비, 누가기록 …)을 바탕화면 우측 하단에 작게 띄워 두고, 끝날 때마다 체크해서 지우는 가벼운 Windows 프로그램입니다.

## 설치

| 방식 | 파일 | 특징 |
|---|---|---|
| 설치형 | `G-routine_0.1.0_x64-setup.exe` | 관리자 권한 없이 설치. 처음 실행할 때 데이터 저장 폴더를 고릅니다 |
| 포터블 | `G-routine_0.1.0_portable.zip` | 압축을 풀어 바로 실행. 데이터는 같은 폴더의 `data\`에 저장됩니다 |

### 복원 프로그램이 있는 학교 PC
재부팅하면 C드라이브가 초기화되는 PC라면 다음 순서를 따르세요.
1. 복원 프로그램을 **해제한 상태**에서 설치형으로 설치하고 한 번 실행합니다 (자동 시작 등록이 유지됩니다).
2. 저장 위치는 **D드라이브**(기본값 `D:\G-routine\data`)로 둡니다.
3. 복원을 다시 켜도 루틴과 체크 기록은 D드라이브에 남습니다. 혹시 설정이 지워져도 G-routine이 `D:\G-routine\data`를 자동으로 다시 찾습니다.

또는 포터블 zip을 D드라이브에 풀어서 쓰면 C드라이브를 전혀 쓰지 않습니다. 단, 이 경우 자동 시작은 복원 해제 상태에서 켜야 유지됩니다.

## 사용법
- 동그라미를 누르면 완료 → 3초 안에 "되돌리기" 가능
- ↗ 버튼: 루틴에 연결한 사이트나 프로그램(NEIS 등) 열기
- 아래 입력란: 오늘 하루만 할 일을 빠르게 추가 (그날이 지나면 미완료로 기록되고 사라짐)
- ⚙: 관리 창 (루틴 관리 · 완료 기록 · 설정)
- 트레이 아이콘: 위젯 보이기, 맨 위 고정, 자동 시작, 위치 초기화, 종료
- 하루는 기본적으로 **오전 4시**에 바뀝니다 (설정에서 변경)

## 개발

```bash
npm install
npm run tauri dev      # 개발 실행
npm test               # 화면 테스트 (Vitest)
cd src-tauri && cargo test   # 규칙 · 저장 테스트
npm run tauri build    # 설치 파일 빌드
powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1   # 포터블 zip
```

브라우저에서 `npm run dev` 후 `http://localhost:1420/`(위젯), `?window=manager`(관리 창), `?setup=1`(첫 실행)로 가짜 데이터 화면을 볼 수 있습니다.

설계 문서: `docs/superpowers/specs/2026-10-05-g-routine-design.md`
````

- [ ] **Step 7: 커밋**

```bash
git add -A
git commit -m "build: release packaging, portable zip script, README and manual checklist

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Spec Coverage

| 스펙 | 구현 Task |
|---|---|
| F1 오늘 목록 · 체크 시 사라짐 | 4 (`get_today`), 8 (`TodayPanel`, `TaskRow`) |
| F2 되돌리기 3초 | 8 (`UndoToast`) |
| F3 완료 항목 보기 · 해제 | 8 (`DoneRow`) |
| F4 진행률 · 모두 완료 | 8 |
| F5 루틴 추가 · 삭제 · 순서 | 3, 4, 9 |
| F6 반복 규칙 | 2, 3, 9 |
| F7 빠른 추가 | 4 (`quick_add`), 8 (`QuickAdd`) |
| F8 하루 시작 시각 | 2, 3, 10 |
| F9 주말 숨김 | 2, 4, 8, 10 |
| F10 날짜별 기록 · 미완료 표시 | 4, 10 (`HistoryTab`) |
| F11 항상 맨 위 | 6 (`shell`), 8 (핀), 10 (설정) |
| F12 자동 시작 | 6 (`sync_autostart`), 10 |
| F13 트레이 | 6 (`tray.rs`) |
| F14 위치 기억 · 화면 밖 복귀 | 6 (`position.rs`, `window.rs`) |
| F15 저장 폴더 선택 · D드라이브 안내 | 5, 6, 8 (`Setup`) |
| F16 기본 템플릿 | 4 (`templates.rs`), 8 |
| F17 백업 / 불러오기 | 5 (`export.rs`), 10 |
| F18 바로가기 | 3 (`is_allowed_link`), 6 (`open_link`), 8, 9 |
| 5.3 저장 위치 해석 · 자동 재연결 | 5, 6 (`startup::boot`) |
| 7 오류 처리 (폴더 없음 · 손상 · 쓰기 실패 · 중복 실행 · 링크 실패 · 화면 밖) | 5, 6, 8 |
| 8 테스트 | 2~10 단위 테스트, 11 스모크 · 수동 점검표 |
| 9 성공 기준 (크기 · 메모리) | 11 Step 3~4 |
| 3.3 배포 중 포터블 exe | 11 (설치형은 MVP에 포함, 자동 업데이트는 2단계 이후) |
