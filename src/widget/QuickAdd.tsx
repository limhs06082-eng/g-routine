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
