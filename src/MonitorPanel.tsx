import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Monitor, RefreshCw, Sun } from "lucide-react";

type Display = {
  id: string;
  name: string;
  primary: boolean;
  x: number;
  y: number;
  brightness: number | null;
  error: string | null;
};
const examples: Display[] = [
  {
    id: "demo-display-1",
    name: "모니터 A",
    primary: true,
    x: 0,
    y: 0,
    brightness: 65,
    error: null,
  },
  {
    id: "demo-display-2",
    name: "모니터 B",
    primary: false,
    x: 2560,
    y: 0,
    brightness: 40,
    error: null,
  },
];

function BrightnessSlider({
  label,
  value,
  disabled,
  onChange,
  onCommit,
}: {
  label: string;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}) {
  return (
    <div className="monitor-slider">
      <Sun size={15} aria-hidden="true" />
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        aria-label={label}
        aria-valuetext={`${value}%`}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        onPointerUp={(e) => onCommit(Number(e.currentTarget.value))}
        onKeyUp={(e) => {
          if (
            [
              "ArrowLeft",
              "ArrowRight",
              "ArrowUp",
              "ArrowDown",
              "Home",
              "End",
              "PageUp",
              "PageDown",
            ].includes(e.key)
          )
            onCommit(Number(e.currentTarget.value));
        }}
        onBlur={(e) => onCommit(Number(e.currentTarget.value))}
      />
      <output>
        {value}
        <span>%</span>
      </output>
    </div>
  );
}

export default function MonitorPanel({ demo }: { demo: boolean }) {
  const [displays, setDisplays] = useState<Display[]>([]);
  const [drafts, setDrafts] = useState<Record<string, number>>({});
  const [allDraft, setAllDraft] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const lock = useRef(false);
  const available = demo || isTauri();
  const supported = displays.filter((display) => display.brightness !== null);
  const average = supported.length
    ? Math.round(
        supported.reduce((sum, display) => sum + display.brightness!, 0) /
          supported.length,
      )
    : 50;

  async function refresh() {
    if (lock.current || !available) return;
    lock.current = true;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const result = demo
        ? examples.map((display) => ({ ...display }))
        : await invoke<Display[]>("scan_monitors");
      setDisplays(result);
      setDrafts({});
      setAllDraft(null);
      setLoaded(true);
    } catch (e) {
      setError(String(e));
    } finally {
      lock.current = false;
      setBusy(false);
    }
  }
  useEffect(() => {
    void refresh();
  }, []);

  async function apply(targets: Display[], value: number, all = false) {
    if (lock.current || !available || targets.length === 0) return;
    if (
      all
        ? allDraft === null
        : targets.every((display) => display.brightness === value)
    )
      return;
    lock.current = true;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const result = demo
        ? targets.map((display) => ({
            ...display,
            brightness: value,
            error: null,
          }))
        : await invoke<Display[]>("set_monitor_brightness", {
            ids: targets.map((display) => display.id),
            brightness: value,
          });
      setDisplays((current) =>
        current.map(
          (display) => result.find((item) => item.id === display.id) ?? display,
        ),
      );
      const failed = result.filter((display) => display.error);
      setMessage(
        `${demo ? "데모: " : ""}${result.length - failed.length}/${result.length}개 모니터 밝기를 적용했습니다.`,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setDrafts({});
      setAllDraft(null);
      lock.current = false;
      setBusy(false);
    }
  }

  return (
    <section
      className="monitors"
      aria-labelledby="monitors-title"
      aria-busy={busy}
    >
      <div className="section-heading">
        <h2 id="monitors-title">
          모니터 밝기 {loaded && <span>{displays.length}</span>}
        </h2>
        <button
          className="text-button"
          disabled={busy || !available}
          onClick={() => void refresh()}
        >
          <RefreshCw size={13} className={busy ? "spinning" : ""} /> 다시 검색
        </button>
      </div>
      <p className="monitor-hint">
        슬라이더를 놓으면 적용됩니다. 0%는 모니터의 최소 밝기입니다.
      </p>
      {!available && (
        <p className="monitor-hint">
          밝기 제어는 Windows 앱에서 사용할 수 있습니다.
        </p>
      )}
      {busy && !loaded && (
        <p className="monitor-hint" role="status">
          연결된 모니터의 밝기를 읽고 있습니다…
        </p>
      )}
      {loaded && displays.length === 0 && (
        <p className="monitor-hint">
          연결된 모니터가 없습니다. 연결 후 다시 검색하세요.
        </p>
      )}
      {supported.length > 1 && (
        <div className="monitor-all">
          <div className="monitor-all-heading">
            전체 밝기 맞추기 <span>지원 모니터 {supported.length}대</span>
          </div>
          <BrightnessSlider
            label="전체 모니터 밝기"
            value={allDraft ?? average}
            disabled={busy}
            onChange={setAllDraft}
            onCommit={(value) => void apply(supported, value, true)}
          />
        </div>
      )}
      <ul className="monitor-list">
        {displays.map((display, index) => (
          <li key={display.id}>
            <div className="monitor-heading">
              <Monitor size={18} aria-hidden="true" />
              <h3>{display.name}</h3>
              <span>
                {index + 1}
                {display.primary ? " · 주 모니터" : " · 보조 모니터"}
              </span>
            </div>
            {display.brightness !== null && (
              <BrightnessSlider
                label={`${index + 1}번 ${display.name} 밝기`}
                value={drafts[display.id] ?? display.brightness}
                disabled={busy}
                onChange={(value) =>
                  setDrafts((current) => ({ ...current, [display.id]: value }))
                }
                onCommit={(value) => void apply([display], value)}
              />
            )}
            {display.error && (
              <p className="monitor-error" role="alert">
                {display.error}
              </p>
            )}
          </li>
        ))}
      </ul>
      <p className="monitor-status" role="status">
        {busy && loaded ? "모니터와 통신 중…" : message}
      </p>
      {error && (
        <p className="monitor-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
