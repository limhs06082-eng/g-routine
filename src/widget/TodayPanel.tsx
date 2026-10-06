import { useCallback, useState } from "react";
import { ChevronDown, ChevronUp, Coffee, PartyPopper } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api, errorMessage, type DayItem, type Rest, type TodayView } from "@/lib/api";
import { useData } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { DoneRow } from "./DoneRow";
import { MiniPill } from "./MiniPill";
import { Notice } from "./Notice";
import { QuickAdd } from "./QuickAdd";
import { TaskRow } from "./TaskRow";
import { UndoToast } from "./UndoToast";
import { WidgetHeader } from "./WidgetHeader";
import { formatDayLabel, groupBySlot, localTimestamp, markDone, markPending, progress, SLOT_TITLES } from "./today";

const LEAVE_MS = 220;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** 받침이 있으면 "이에요", 없으면 "예요" (괄호 등은 건너뛰고 마지막 한글로 판단) */
function iyeyo(word: string): string {
  const last = [...word].reverse().find((ch) => ch >= "가" && ch <= "힣");
  if (!last) return "이에요";
  return (last.charCodeAt(0) - 0xac00) % 28 === 0 ? "예요" : "이에요";
}

function restMessage(rest: Rest): string {
  if (rest.kind === "holiday") return `오늘은 ${rest.name}${iyeyo(rest.name)}. 푹 쉬세요`;
  if (rest.kind === "vacation") return "방학 중이에요. 푹 쉬세요";
  return "좋은 주말 보내세요";
}

function EmptyState({ view, total }: { view: TodayView; total: number }) {
  if (total > 0) {
    return (
      <div className="flex flex-col items-center gap-1 px-4 py-5 text-center text-xs text-muted-foreground">
        <PartyPopper className="size-5 text-primary" />
        <span className="font-medium text-foreground">오늘 루틴을 모두 마쳤어요</span>
      </div>
    );
  }
  if (view.rest) {
    return (
      <div className="flex flex-col items-center gap-1 px-4 py-5 text-center text-xs text-muted-foreground">
        <Coffee className="size-5 text-primary" />
        <span className="font-medium text-foreground">{restMessage(view.rest)}</span>
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

interface Props {
  pinned: boolean;
  onTogglePin: () => Promise<unknown>;
  /** 미니 모드(알약 모양)로 보여 줄지 */
  mini: boolean;
  onToggleMini: () => Promise<unknown>;
}

export function TodayPanel({ pinned, onTogglePin, mini, onToggleMini }: Props) {
  const { data, error, reload, setData } = useData(api.today);
  const [leaving, setLeaving] = useState<number[]>([]);
  const [undoItem, setUndoItem] = useState<DayItem | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [showDone, setShowDone] = useState(false);
  const closeUndo = useCallback(() => setUndoItem(null), []);
  const closeNotice = useCallback(() => setNotice(null), []);

  if (!data) {
    if (!error) return <div className={mini ? "h-8" : "h-28"} />;
    return (
      <div className="flex flex-col items-center gap-2 px-4 py-6 text-center text-xs text-muted-foreground">
        <span>오늘 목록을 불러오지 못했어요.</span>
        <Button size="sm" variant="outline" onClick={() => void reload()}>
          다시 시도
        </Button>
      </div>
    );
  }
  const view = data;
  if (mini) return <MiniPill view={view} onExpand={() => void onToggleMini().catch(() => {})} />;
  const { done, total, percent } = progress(view);
  const allDone = total > 0 && done === total;
  const groups = groupBySlot(view.pending);
  const row = (item: DayItem) => (
    <TaskRow
      key={item.id}
      item={item}
      leaving={leaving.includes(item.id)}
      onCheck={() => void check(item)}
      onOpenLink={() => void openLink(item)}
    />
  );

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
      void reload();
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
      void reload();
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
      <WidgetHeader
        dayLabel={formatDayLabel(view.day)}
        pinned={pinned}
        onTogglePin={() => void onTogglePin().catch((e) => setNotice(errorMessage(e)))}
        onShrink={() => void onToggleMini().catch((e) => setNotice(errorMessage(e)))}
      />
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
        groups ? (
          <div data-testid="pending-list" className="px-1">
            {groups.map((g) => (
              <section key={g.slot ?? "none"} aria-label={SLOT_TITLES[g.slot ?? "none"]}>
                <div className="px-2.5 pt-1.5 pb-0.5 text-[11px] font-medium text-muted-foreground">{SLOT_TITLES[g.slot ?? "none"]}</div>
                <ul>{g.items.map(row)}</ul>
              </section>
            ))}
          </div>
        ) : (
          <ul data-testid="pending-list" className="px-1">
            {view.pending.map(row)}
          </ul>
        )
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
            <ul className="px-1 pb-1">
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
