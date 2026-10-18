import { useEffect, useState } from "react";
import { Cloud, FolderOpen } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { api, type Backup2Info } from "@/lib/api";

interface Props {
  /** SettingsTab의 실행 도우미: 실패하면 오류 문구를, 성공하면 ok 문구를 보여 준다 */
  run: (action: () => Promise<unknown>, ok?: string) => Promise<void>;
}

/** 두 번째 백업: OneDrive · USB 같은 다른 곳에 주 1회 백업해 두면 PC나 D드라이브가 고장 나도 되살릴 수 있다 */
export function SecondBackup({ run }: Props) {
  const [info, setInfo] = useState<Backup2Info | null>(null);
  useEffect(() => {
    api
      .backup2Info()
      .then(setInfo)
      .catch(() => setInfo(null));
  }, []);

  const use = (dir: string | null, ok: string) =>
    run(async () => {
      setInfo(await api.setBackup2Dir(dir));
    }, ok);
  const pick = () =>
    run(async () => {
      const picked = await open({ directory: true, title: "두 번째 백업 폴더 선택 (OneDrive · USB 등)" });
      if (typeof picked !== "string") return false;
      setInfo(await api.setBackup2Dir(picked));
    }, "두 번째 백업 폴더를 정하고 한 번 백업했어요");
  const now = () =>
    run(async () => {
      setInfo(await api.backup2Now());
    }, "지금 백업했어요");

  if (!info) return null;
  return (
    <div className="border-b border-border py-3">
      <div className="text-sm">두 번째 백업 (주 1회)</div>
      {info.dir ? (
        <>
          <div className="text-xs text-muted-foreground">
            <span className="font-mono">{info.dir}\G-routine 백업</span> 폴더에 일주일마다 백업해요 · 마지막{" "}
            {info.last ?? "아직 없음"}
          </div>
          <div className="mt-2 flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={() => void now()}>
              지금 백업
            </Button>
            <Button size="sm" variant="ghost" onClick={() => void pick()}>
              <FolderOpen />
              폴더 바꾸기
            </Button>
            <Button size="sm" variant="ghost" onClick={() => void use(null, "두 번째 백업을 껐어요")}>
              끄기
            </Button>
          </div>
          <p className="mt-1.5 text-xs text-muted-foreground">되살릴 때는 아래 '불러오기'에서 그 폴더의 파일을 고르세요.</p>
        </>
      ) : (
        <>
          <div className="text-xs text-muted-foreground">
            PC나 D드라이브가 고장 나도 기록을 되살릴 수 있게, 다른 곳에 일주일마다 백업해 둬요.
          </div>
          <div className="mt-2 flex flex-wrap gap-2">
            {info.suggested && (
              <Button size="sm" onClick={() => void use(info.suggested, "OneDrive에 백업해 두었어요. 앞으로 일주일마다 백업해요")}>
                <Cloud />
                OneDrive에 백업하기
              </Button>
            )}
            <Button size="sm" variant={info.suggested ? "ghost" : "outline"} onClick={() => void pick()}>
              <FolderOpen />
              {info.suggested ? "다른 폴더 고르기" : "폴더 고르기 (USB · OneDrive 등)"}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
