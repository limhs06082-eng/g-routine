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
