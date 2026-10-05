import { api } from "@/lib/api";
import { useData, useSettings } from "@/lib/hooks";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { HistoryTab } from "./HistoryTab";
import { RoutinesTab } from "./RoutinesTab";
import { SettingsTab } from "./SettingsTab";

export function ManagerApp() {
  const { data: status } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="flex items-center gap-2.5 border-b border-border px-6 py-4">
        <div className="size-6 rounded-lg bg-soft-3" />
        <h1 className="text-base font-semibold">G-routine 관리</h1>
      </header>
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
