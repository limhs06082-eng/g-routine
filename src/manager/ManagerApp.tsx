import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { useData, useSettings } from "@/lib/hooks";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { HistoryTab } from "./HistoryTab";
import { RoutinesTab } from "./RoutinesTab";
import { SettingsTab } from "./SettingsTab";

export function ManagerApp() {
  const { data: status, error, reload } = useData(api.status);

  // 파일을 창에 떨어뜨려도 WebView가 그 파일로 이동하지 않게 막는다 (행 끌어 옮기기는 그대로 동작)
  useEffect(() => {
    const block = (e: DragEvent) => e.preventDefault();
    document.addEventListener("dragover", block);
    document.addEventListener("drop", block);
    return () => {
      document.removeEventListener("dragover", block);
      document.removeEventListener("drop", block);
    };
  }, []);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="flex items-center gap-2.5 border-b border-border px-6 py-4">
        <div className="size-6 rounded-lg bg-soft-3" />
        <h1 className="text-base font-semibold">G-routine 관리</h1>
      </header>
      {!status && error && (
        <div className="flex items-center gap-2 p-6 text-sm text-muted-foreground">
          <span>G-routine 상태를 불러오지 못했어요.</span>
          <Button size="sm" variant="outline" onClick={() => void reload()}>
            다시 시도
          </Button>
        </div>
      )}
      {status && !ready && <p className="p-6 text-sm text-muted-foreground">먼저 위젯에서 시작 설정을 마쳐 주세요.</p>}
      {status && ready && (
        <Tabs defaultValue="routines" className="px-6 py-4">
          <TabsList>
            <TabsTrigger value="routines">루틴 관리</TabsTrigger>
            <TabsTrigger value="history">완료 기록</TabsTrigger>
            <TabsTrigger value="settings">설정</TabsTrigger>
          </TabsList>
          <TabsContent value="routines">
            <RoutinesTab />
          </TabsContent>
          <TabsContent value="history">
            <HistoryTab />
          </TabsContent>
          <TabsContent value="settings">
            {settings && <SettingsTab settings={settings} status={status} onChange={update} />}
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}
