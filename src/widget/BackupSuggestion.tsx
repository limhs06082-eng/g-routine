import { useState } from "react";
import { Cloud } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api, errorMessage } from "@/lib/api";

/** OneDrive가 있는데 두 번째 백업을 정하지 않은 PC에 한 번 권한다. '나중에'를 누르면 위젯에서는 다시 묻지 않는다 */
export function BackupSuggestion({ dir, onDone }: { dir: string; onDone: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function act(action: () => Promise<unknown>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      onDone();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-label="OneDrive 백업 권유" className="mx-2.5 mb-2 rounded-lg bg-soft/60 px-3 py-2.5 text-xs">
      <div className="mb-1 flex items-center gap-1 font-medium text-ink">
        <Cloud className="size-3.5 text-primary" />
        OneDrive에 백업해 둘까요?
      </div>
      <p className="leading-5 text-muted-foreground">PC가 고장 나도 기록을 되살릴 수 있게 일주일마다 OneDrive에 백업해요.</p>
      {error && (
        <p role="alert" className="mt-1 text-danger">
          {error}
        </p>
      )}
      <div className="mt-1.5 flex justify-end gap-1.5">
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => void act(api.dismissBackup2)}>
          나중에
        </Button>
        <Button size="sm" disabled={busy} onClick={() => void act(() => api.setBackup2Dir(dir))}>
          백업하기
        </Button>
      </div>
    </section>
  );
}
