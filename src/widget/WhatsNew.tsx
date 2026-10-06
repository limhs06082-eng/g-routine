import { Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";

/** 업데이트 뒤 한 번 보여 주는 '바뀐 점' 카드 */
export function WhatsNew({ notes, onClose }: { notes: { version: string; items: string[] }[]; onClose: () => void }) {
  return (
    <section aria-label="새 버전에서 바뀐 점" className="mx-2.5 mb-2 rounded-lg bg-soft/60 px-3 py-2.5 text-xs">
      <div className="mb-1 flex items-center gap-1 font-medium text-ink">
        <Sparkles className="size-3.5 text-primary" />
        v{notes[0].version}에서 바뀐 점
      </div>
      <ul className="list-disc space-y-0.5 pl-4 leading-5 text-muted-foreground">
        {notes.flatMap((n) => n.items).map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
      <div className="mt-1.5 flex justify-end">
        <Button size="sm" variant="outline" onClick={onClose}>
          확인
        </Button>
      </div>
    </section>
  );
}
