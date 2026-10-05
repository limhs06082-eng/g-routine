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
