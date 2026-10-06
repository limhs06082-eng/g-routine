import { useEffect, useState, type ReactNode } from "react";
import { Download, FolderOpen, Info, Upload } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
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
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);
  const [shownDir, setShownDir] = useState<string | null>(null);
  const dataDir = shownDir ?? status.dataDir;
  useEffect(() => setShownDir(null), [status.dataDir]);

  /** 대화상자 호출까지 포함해 실행한다. action이 false를 돌려주면(사용자가 취소) 성공 메시지를 띄우지 않는다. */
  async function run(action: () => Promise<unknown>, ok?: string) {
    try {
      const result = await action();
      setMessage(ok && result !== false ? { ok: true, text: ok } : null);
    } catch (e) {
      setMessage({ ok: false, text: errorMessage(e) });
    }
  }

  const toggle = (key: SettingKey) => (checked: boolean) => void run(() => onChange(key, String(checked)));

  const changeDir = () =>
    run(async () => {
      const picked = await open({ directory: true, title: "새 저장 폴더 선택" });
      if (typeof picked !== "string") return false;
      const info = await api.inspectDataDir(picked);
      if (info.hasData) {
        const ok = await confirm(
          "선택한 폴더에 이미 G-routine 데이터가 있어요. 지금 데이터 대신 그 데이터를 사용할까요? 지금 데이터는 원래 폴더에 그대로 남아요.",
          { title: "저장 위치 변경", kind: "warning" },
        );
        if (!ok) return false;
      }
      await api.changeDataDir(info.normalized);
      // 상태가 새로 읽힐 때까지 실제로 쓰이는 경로(…\G-routine\data)를 먼저 보여 준다
      setShownDir(info.normalized);
    }, "저장 위치를 바꿨어요");

  const exportBackup = () =>
    run(async () => {
      const path = await save({ defaultPath: `g-routine-backup-${todayString()}.json`, filters: JSON_FILTER });
      if (!path) return false;
      await api.exportBackup(path);
    }, "백업 파일을 저장했어요");

  const importBackup = () =>
    run(async () => {
      const path = await open({ multiple: false, directory: false, filters: JSON_FILTER });
      if (typeof path !== "string") return false;
      const ok = await confirm("지금 데이터를 백업 파일 내용으로 바꿀까요? 되돌릴 수 없어요.", {
        title: "백업 불러오기",
        kind: "warning",
      });
      if (!ok) return false;
      await api.importBackup(path);
    }, "백업을 불러왔어요");

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
          {Array.from({ length: 24 }, (_, h) => (
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
          <div className="min-w-0 flex-1 truncate rounded-lg bg-muted px-3 py-2 font-mono text-xs" title={dataDir ?? ""}>
            {status.portable ? "포터블 모드 · 프로그램 폴더의 data" : dataDir}
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
      <p className="mt-6 text-xs text-muted-foreground">
        G-routine {version ? `v${version}` : ""} ·{" "}
        {status.portable
          ? "포터블 버전은 자동으로 업데이트되지 않아요. 릴리스 페이지에서 새 zip을 받아 주세요."
          : "새 버전이 나오면 자동으로 설치되고 다시 켜져요."}
      </p>
    </div>
  );
}
