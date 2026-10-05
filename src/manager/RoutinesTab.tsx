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
  const { data, error, reload, setData } = useData(api.listRoutines);
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
    setNotice(null);
    const next = moveItem(routines, dragFrom, to);
    setData(next);
    setDragFrom(null);
    try {
      await api.reorderRoutines(next.map((r) => r.id));
    } catch (e) {
      setNotice(errorMessage(e));
      await reload();
    }
  }

  async function remove(id: number) {
    setNotice(null);
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
          내 루틴 {data && <span className="font-normal text-muted-foreground">{routines.length}개</span>}
        </h2>
        {data === null ? (
          error && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span>루틴 목록을 불러오지 못했어요.</span>
              <Button size="sm" variant="outline" onClick={() => void reload()}>
                다시 시도
              </Button>
            </div>
          )
        ) : routines.length === 0 ? (
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
