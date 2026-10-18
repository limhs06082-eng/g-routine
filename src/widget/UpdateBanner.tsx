import { useState } from "react";
import { Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api, errorMessage } from "@/lib/api";

/**
 * 주기 확인에서 찾은 새 버전을 알리고 '지금 설치'를 받는다.
 * 수업 중에 앱이 갑자기 꺼지지 않도록, 켜진 직후가 아닌 때 찾은 버전은 사용자가 누를 때만 설치한다.
 * 설치가 시작되면 앱이 꺼졌다 다시 켜지므로 '설치하는 중…'은 그때까지만 보인다.
 */
export function UpdateBanner({ version }: { version: string }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function install() {
    setBusy(true);
    setError(null);
    try {
      await api.installUpdate();
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  }

  return (
    <section aria-label="새 버전 안내" className="mx-2.5 mb-2 rounded-lg bg-soft/60 px-3 py-2 text-xs">
      <div className="flex flex-wrap items-center justify-between gap-x-2 gap-y-1">
        <span className="flex items-center gap-1 text-ink">
          <Download className="size-3.5 shrink-0 text-primary" />
          새 버전 v{version} 준비됐어요
        </span>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => void install()}>
          {busy ? "설치하는 중…" : "지금 설치"}
        </Button>
      </div>
      <p className="mt-1 text-[11px] leading-4 text-muted-foreground">설치하는 동안 잠깐 꺼졌다 다시 켜져요. 수업 중이면 나중에 눌러도 돼요.</p>
      {error && (
        <p role="alert" className="mt-1 text-[11px] text-danger">
          {error}
        </p>
      )}
    </section>
  );
}
