import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";

type Props = { busy: boolean; onError: (message: string) => void };

export default function Titlebar({ busy, onError }: Props) {
  const [maximized, setMaximized] = useState(false);
  const native = isTauri();

  useEffect(() => {
    if (!native) return;
    const window = getCurrentWindow();
    let disposed = false;
    const sync = async () => {
      try {
        const value = await window.isMaximized();
        if (!disposed) setMaximized(value);
      } catch {
        // The buttons remain usable if a resize query races with window closure.
      }
    };
    void sync();
    const unsubscribe = window.onResized(() => void sync()).catch(() => null);
    return () => {
      disposed = true;
      void unsubscribe.then((unlisten) => unlisten?.());
    };
  }, [native]);

  async function act(action: "minimize" | "toggleMaximize" | "close") {
    if (!native || (action === "close" && busy)) return;
    try {
      await getCurrentWindow()[action]();
    } catch {
      onError("창을 조작하지 못했습니다. 잠시 후 다시 시도하세요.");
    }
  }

  return (
    <header className="titlebar">
      <div className="titlebar-drag" data-tauri-drag-region>
        <img
          className="app-mark"
          src="/assets/app-icon.png"
          alt=""
          draggable={false}
        />
        <span className="app-name">RGB Switch</span>
      </div>
      <div className="window-controls" role="group" aria-label="창 제어">
        <button
          aria-label="최소화"
          title={native ? "최소화" : "앱에서 사용할 수 있습니다"}
          disabled={!native}
          onClick={() => void act("minimize")}
        >
          <Minus size={15} />
        </button>
        <button
          aria-label={maximized ? "이전 크기로 복원" : "최대화"}
          title={
            native
              ? maximized
                ? "이전 크기로 복원"
                : "최대화"
              : "앱에서 사용할 수 있습니다"
          }
          disabled={!native}
          onClick={() => void act("toggleMaximize")}
        >
          {maximized ? <Copy size={13} /> : <Square size={12} />}
        </button>
        <button
          className="window-close"
          aria-label="트레이로 숨기기"
          title={
            busy
              ? "장치 명령 처리 중"
              : native
                ? "트레이로 숨기기"
                : "앱에서 사용할 수 있습니다"
          }
          disabled={!native || busy}
          onClick={() => void act("close")}
        >
          <X size={17} />
        </button>
      </div>
    </header>
  );
}
