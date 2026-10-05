import { useState } from "react";
import { FolderOpen, Info } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { api, errorMessage, type AppStatus, type TemplateName } from "@/lib/api";
import { normalizeDataDir } from "@/lib/dataDir";
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
    if (typeof picked !== "string") return;
    setDir(normalizeDataDir(picked));
    try {
      // 이미 데이터가 있는 폴더면 그대로 쓰는지 등은 앱(Rust)이 최종 판단한다
      setDir((await api.inspectDataDir(picked)).normalized);
    } catch {
      // 표시용 경로라 실패해도 위의 값으로 둔다
    }
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

      {status.openFailed && (
        <div className="rounded-lg bg-chip-due p-2.5 text-xs leading-5 text-chip-due-ink">
          데이터 파일을 열 수 없어요. 다른 프로그램이 사용 중일 수 있어요. 잠시 후 다시 시도해 주세요.
          <Button size="sm" variant="outline" className="mt-2 w-full" disabled={busy} onClick={() => void run(api.retryBoot)}>
            다시 시도
          </Button>
        </div>
      )}
      {!status.openFailed && status.corrupt && (
        <div className="rounded-lg bg-danger-soft p-2.5 text-xs text-danger">
          데이터 파일이 손상된 것 같아요. 자동 백업으로 되돌릴 수 있어요.
          <Button size="sm" variant="outline" className="mt-2 w-full" disabled={busy} onClick={() => void run(api.restoreBackup)}>
            최근 백업으로 복구
          </Button>
          <p className="mt-2 leading-5">복구가 안 되면 아래에서 시작하기를 누르세요. 손상된 파일은 따로 보관돼요.</p>
        </div>
      )}
      {!status.corrupt && !status.openFailed && status.previousDir && (
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
          복원 프로그램이 있는 PC는 D드라이브를 고르세요. 고른 폴더 안에 G-routine\data 폴더가 만들어져요.
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
