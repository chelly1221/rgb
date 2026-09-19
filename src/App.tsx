import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import DpiPanel, { type DpiProfile, type MouseStatus } from "./DpiPanel";
import Titlebar from "./Titlebar";
import MonitorPanel from "./MonitorPanel";
import Profiles, {
  deviceKeys,
  defaultDpi,
  defaultLighting,
  type SceneProfile,
} from "./Profiles";
import LightingPanel, {
  effectNames,
  colorHex,
  type Effect,
  type Lighting,
} from "./LightingPanel";
import {
  CircuitBoard,
  Cpu,
  Fan,
  Keyboard,
  Lightbulb,
  Monitor,
  Mouse,
  Power,
  RefreshCw,
  Unplug,
  Settings2,
  Check,
  AlertCircle,
  ChevronDown,
  ArrowUpRight,
  Info,
} from "lucide-react";

type Device = {
  id: number;
  key: string;
  name: string;
  vendor: string;
  kind: number;
  ledCount: number;
  mode: string;
  supported: boolean;
  present?: boolean;
  detail?: string;
  lastRequested: boolean | null;
  effects: Effect[];
  lastLighting?: Lighting | null;
};
type Outcome = { id: number; error: string | null };
type StartupStatus = { enabled: boolean; ready: boolean; error: string | null };

const demoDevices: Device[] = [
  {
    id: 0,
    key: "demo-mouse",
    name: "HARPOON RGB WIRELESS",
    vendor: "데모 장치",
    kind: 6,
    ledCount: 2,
    mode: "USB 유선",
    detail: "데모 마우스입니다. 실제 장치에는 명령을 보내지 않습니다.",
    supported: true,
    lastRequested: null,
    effects: ["static"],
  },
  {
    id: 1,
    key: "demo-gpu",
    name: "Palit GeForce RTX 3070",
    vendor: "데모 장치",
    kind: 1,
    ledCount: 1,
    mode: "그래픽카드 조명",
    detail: "데모 그래픽카드입니다. 실제 장치에는 명령을 보내지 않습니다.",
    supported: true,
    lastRequested: null,
    effects: ["static", "breathe", "spectrum"],
  },
  {
    id: 2,
    key: "demo-board",
    name: "MSI B860M MORTAR",
    vendor: "데모 장치",
    kind: 0,
    ledCount: 0,
    mode: "RGB 헤더",
    detail:
      "데모 메인보드입니다. 연결된 RGB 헤더를 함께 제어하며 기존 효과를 유지합니다.",
    supported: true,
    lastRequested: null,
    effects: ["static", "breathe", "spectrum"],
  },
  {
    id: 3,
    key: "demo-keyboard",
    name: "GS3087T",
    vendor: "데모 장치",
    kind: 5,
    ledCount: 0,
    mode: "USB 유선",
    detail:
      "데모 키보드입니다. 실제 앱에서는 USB 유선 연결로 색상과 효과를 제어합니다.",
    supported: true,
    lastRequested: null,
    effects: ["static", "breathe", "spectrum"],
  },
  {
    id: 4,
    key: "demo-memory",
    name: "VENGEANCE RGB DDR5 · 2개",
    vendor: "데모 장치",
    kind: 2,
    ledCount: 20,
    mode: "메모리 조명",
    detail:
      "데모 메모리입니다. 실제 제어에는 관리자 권한과 앱에 내장된 메모리 드라이버가 필요합니다.",
    supported: true,
    lastRequested: null,
    effects: ["static", "breathe", "spectrum"],
  },
];

function deviceKind(kind: number) {
  switch (kind) {
    case 0:
      return { Icon: CircuitBoard, label: "메인보드" };
    case 1:
      return { Icon: Monitor, label: "그래픽카드" };
    case 2:
      return { Icon: Cpu, label: "메모리" };
    case 3:
    case 4:
    case 12:
      return { Icon: Fan, label: "쿨링 / 조명" };
    case 5:
      return { Icon: Keyboard, label: "키보드" };
    case 6:
      return { Icon: Mouse, label: "마우스" };
    default:
      return { Icon: Lightbulb, label: "RGB 장치" };
  }
}

export default function App() {
  const [startup, setStartup] = useState<StartupStatus>({
    enabled: false,
    ready: false,
    error: null,
  });
  const [startupBusy, setStartupBusy] = useState(false);
  const [mouseTab, setMouseTab] = useState<"lighting" | "dpi">("lighting");
  const [mouseStatus, setMouseStatus] = useState<MouseStatus | null>(null);
  const [demoDpi, setDemoDpi] = useState<DpiProfile>({
    values: [400, 1000, 1600, 3000, 5000],
    colors: [
      [255, 0, 0],
      [255, 255, 255],
      [0, 255, 0],
      [255, 255, 0],
      [0, 191, 255],
    ],
    mask: 31,
    index: 2,
  });
  const [liveDevices, setDevices] = useState<Device[]>([]);
  const [profileDraft, setProfileDraft] = useState<SceneProfile | null>(null);
  const [profileRevision, setProfileRevision] = useState(0);
  const devices: Device[] = profileDraft
    ? demoDevices.map((template) => {
        const entry = profileDraft.devices.find((e) => e.id === template.id)!;
        return {
          ...template,
          key: entry.key,
          vendor: "프로파일 편집",
          detail:
            "프로파일에 저장할 설정입니다. 저장·적용을 누르면 장치에 반영됩니다.",
          lastRequested: entry.enabled,
          lastLighting: entry.lighting,
        };
      })
    : liveDevices;
  const [connected, setConnected] = useState(false);
  const [demo, setDemo] = useState(false);
  const [commandBusy, setBusy] = useState<string | null>(null);
  const [profileSaving, setProfileSaving] = useState(false);
  const busy = commandBusy ?? (profileSaving ? "프로파일 저장 중…" : null);
  const busyRef = useRef(false);
  const [settings, setSettings] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [filter, setFilter] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [failures, setFailures] = useState<Record<number, string>>({});
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const subscription = listen<StartupStatus>(
      "startup-status",
      ({ payload }) => {
        if (!disposed) setStartup(payload);
      },
    );
    void subscription
      .then(() => invoke<StartupStatus>("get_startup"))
      .then((status) => {
        if (!disposed && status.ready) setStartup(status);
      })
      .catch((e) => {
        if (!disposed)
          setStartup({ enabled: false, ready: true, error: String(e) });
      });
    return () => {
      disposed = true;
      void subscription.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  async function toggleStartup(enabled: boolean) {
    if (startupBusy) return;
    setStartupBusy(true);
    try {
      setStartup(await invoke<StartupStatus>("set_startup", { enabled }));
    } catch (e) {
      setStartup((current) => ({ ...current, error: String(e) }));
    } finally {
      setStartupBusy(false);
    }
  }
  useEffect(() => {
    if (!isTauri() || demo) return;
    let disposed = false;
    const subscription = listen<MouseStatus>("mouse-status", ({ payload }) => {
      if (disposed) return;
      setMouseStatus(payload);
      if (payload.error) {
        setError(payload.error);
        setMessage("");
        setFailures((current) => ({ ...current, 0: payload.error! }));
        setDevices((current) =>
          current.map((d) => (d.id === 0 ? { ...d, lastRequested: null } : d)),
        );
      }
    }).catch((e) => {
      if (!disposed)
        setError(`마우스 상태 알림을 연결하지 못했습니다: ${String(e)}`);
      return null;
    });
    return () => {
      disposed = true;
      void subscription.then((unlisten) => unlisten?.());
    };
  }, [demo]);
  const supported = devices.filter((d) => d.supported);
  const available = connected || demo || !!profileDraft;

  function editProfile(profile: SceneProfile | null) {
    setProfileDraft(profile);
    setFilter("");
    if (!profileDraft && profile) {
      setExpanded(profile.devices[0].key);
      setMouseTab("lighting");
    }
  }
  function newProfile(): SceneProfile {
    return {
      id: crypto.randomUUID(),
      name: "",
      mouseDpi: structuredClone(
        mouseStatus?.profile ?? (demo ? demoDpi : null),
      ),
      devices: deviceKeys.map((key, id) => {
        const source = liveDevices.find((d) => d.id === id);
        return {
          id,
          key,
          enabled: source?.lastRequested ?? true,
          lighting: structuredClone(source?.lastLighting ?? defaultLighting),
        };
      }),
    };
  }
  function editProfileLighting(id: number, lighting: Lighting) {
    setProfileDraft(
      (p) =>
        p && {
          ...p,
          devices: p.devices.map((e) => (e.id === id ? { ...e, lighting } : e)),
        },
    );
  }
  async function applyProfile(profile: SceneProfile) {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(`‘${profile.name}’ 전체 장치에 적용 중…`);
    setError("");
    setMessage("");
    setFailures({});
    try {
      const outcomes: Outcome[] = demo
        ? profile.devices.map((e) => ({ id: e.id, error: null }))
        : await invoke("apply_profile", { profile });
      setDevices((current) =>
        demoDevices.map((template) => {
          const entry = profile.devices.find((e) => e.id === template.id)!;
          const outcome = outcomes.find((o) => o.id === template.id);
          const previous = current.find((d) => d.id === template.id);
          return {
            ...template,
            ...previous,
            key: demo ? template.key : entry.key,
            supported: outcome?.error == null,
            lastRequested: outcome?.error
              ? null
              : entry.enabled && entry.lighting.brightness > 0,
            lastLighting: outcome?.error
              ? previous?.lastLighting
              : entry.lighting,
            detail:
              outcome?.error ??
              (demo ? template.detail : "프로파일 설정 명령을 완료했습니다."),
          };
        }),
      );
      if (!demo) setConnected(true);
      if (demo && profile.mouseDpi) {
        setDemoDpi(profile.mouseDpi);
        setMouseStatus({
          active: true,
          profile: profile.mouseDpi,
          error: null,
        });
      }
      const failed = outcomes.filter((o) => o.error);
      setFailures(Object.fromEntries(failed.map((o) => [o.id, o.error!])));
      if (failed.length)
        setError(
          `${failed.length}개 장치 적용 실패. 프로파일은 저장되어 있습니다. 장치별 오류를 확인한 뒤 일괄 적용을 다시 누르세요.`,
        );
      setMessage(
        `${demo ? "데모: " : ""}‘${profile.name}’ ${outcomes.length - failed.length}/5개 장치 적용 완료.`,
      );
      setProfileRevision((r) => r + 1);
    } catch (e) {
      setError(
        `프로파일 적용 실패: ${String(e)}. 저장한 프로파일로 다시 시도할 수 있습니다.`,
      );
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  }

  async function scan() {
    if (profileDraft) return;
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy("장치를 확인하고 있습니다…");
    setError("");
    setMessage("");
    setFailures({});
    try {
      if (demo) {
        setMessage("데모 장치 목록을 확인했습니다.");
        return;
      }
      if (!isTauri())
        throw new Error(
          "브라우저에서는 실제 장치를 연결할 수 없습니다. RGB Switch 앱을 실행하거나 데모를 사용하세요.",
        );
      setDevices(await invoke<Device[]>("scan_devices"));
      setConnected(true);
      setMessage("장치 목록을 가져왔습니다.");
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
      setDevices([]);
      setConnected(false);
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  }

  async function power(enabled: boolean, targets: Device[]) {
    if (busyRef.current || targets.length === 0) return;
    if (profileDraft) {
      setProfileDraft(
        (p) =>
          p && {
            ...p,
            devices: p.devices.map((e) =>
              targets.some((t) => t.id === e.id) ? { ...e, enabled } : e,
            ),
          },
      );
      return;
    }
    busyRef.current = true;
    setBusy(
      `${targets.length}개 장치에 ${enabled ? "켜기" : "끄기"} 명령을 보내고 있습니다…`,
    );
    setError("");
    setMessage("");
    setFailures({});
    try {
      const outcomes: Outcome[] = demo
        ? targets.map((d) => ({ id: d.id, error: null }))
        : await invoke("set_power", {
            targets: targets.map(({ id, key }) => ({ id, key })),
            enabled,
          });
      const success = outcomes.filter((o) => !o.error).map((o) => o.id);
      setDevices((current) =>
        current.map((d) =>
          outcomes.some((o) => o.id === d.id)
            ? { ...d, lastRequested: success.includes(d.id) ? enabled : null }
            : d,
        ),
      );
      const failed = outcomes.filter((o) => o.error);
      setFailures(Object.fromEntries(failed.map((o) => [o.id, o.error!])));
      if (failed.length)
        setError(
          `${failed.length}개 장치에 명령을 보내지 못했습니다. 장치별 메시지를 확인하고 목록을 새로고침하세요.`,
        );
      setMessage(
        `${demo ? "데모: " : ""}${success.length}개 장치에 ${enabled ? "켜기" : "끄기"} 명령을 보냈습니다.`,
      );
    } catch (e) {
      setError(String(e));
      setDevices((current) =>
        current.map((d) =>
          targets.some((t) => t.id === d.id)
            ? { ...d, lastRequested: null }
            : d,
        ),
      );
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  }

  async function applyLighting(settings: Lighting, ids: number[]) {
    if (busyRef.current) return;
    const targets = devices.filter((d) => ids.includes(d.id) && d.supported);
    if (!targets.length) return;
    busyRef.current = true;
    setBusy("조명 설정을 적용하고 있습니다…");
    setError("");
    setMessage("");
    setFailures({});
    try {
      const outcomes: Outcome[] = demo
        ? targets.map((d) => ({ id: d.id, error: null }))
        : await invoke("set_lighting", {
            targets: targets.map(({ id, key }) => ({ id, key })),
            settings,
          });
      setDevices((current) =>
        current.map((d) => {
          const outcome = outcomes.find((o) => o.id === d.id);
          return outcome
            ? {
                ...d,
                lastRequested: outcome.error ? null : settings.brightness > 0,
                lastLighting: outcome.error ? d.lastLighting : settings,
              }
            : d;
        }),
      );
      const failed = outcomes.filter((o) => o.error);
      setFailures(Object.fromEntries(failed.map((o) => [o.id, o.error!])));
      if (failed.length)
        setError(
          `${failed.length}개 장치의 설정을 완료하지 못했습니다. 장치별 메시지를 확인하세요.`,
        );
      setMessage(
        `${demo ? "데모: " : ""}${outcomes.length - failed.length}개 장치에 ${effectNames[settings.effect]} · 밝기 ${settings.brightness}% 설정을 적용했습니다.`,
      );
    } catch (e) {
      setError(String(e));
      setDevices((current) =>
        current.map((d) =>
          ids.includes(d.id) ? { ...d, lastRequested: null } : d,
        ),
      );
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  }

  async function loadDpi(): Promise<DpiProfile | null> {
    const mouse = devices.find((d) => d.id === 0 && d.supported);
    if (busyRef.current || !mouse) return null;
    busyRef.current = true;
    setBusy("마우스 DPI 설정을 읽고 있습니다…");
    setError("");
    setMessage("");
    try {
      return demo
        ? demoDpi
        : await invoke<DpiProfile>("get_mouse_dpi", { key: mouse.key });
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  }

  async function applyDpi(profile: DpiProfile): Promise<DpiProfile | null> {
    const mouse = devices.find((d) => d.id === 0 && d.supported);
    if (busyRef.current || !mouse) return null;
    busyRef.current = true;
    setBusy("DPI 감도와 표시등을 적용하고 있습니다…");
    setError("");
    setMessage("");
    try {
      const outcome = demo
        ? {
            profile,
            lighting: mouse.lastLighting ?? {
              effect: "static" as Effect,
              color: [255, 164, 211] as [number, number, number],
              brightness: 70,
              speed: 2,
            },
            warning: null,
          }
        : await invoke<{
            profile: DpiProfile;
            lighting: Lighting;
            warning: string | null;
          }>("set_mouse_dpi", { key: mouse.key, profile });
      if (demo) setDemoDpi(profile);
      setMouseStatus({ active: true, profile: outcome.profile, error: null });
      setDevices((current) =>
        current.map((d) =>
          d.id === 0
            ? {
                ...d,
                lastLighting: outcome.lighting,
                lastRequested: outcome.lighting.brightness > 0,
              }
            : d,
        ),
      );
      setFailures((current) => {
        const next = { ...current };
        delete next[0];
        return next;
      });
      if (outcome.warning) {
        setError(outcome.warning);
        return null;
      }
      setMessage(
        `${demo ? "데모: " : ""}DPI 설정을 저장하고 ${outcome.profile.values[outcome.profile.index].toLocaleString()} DPI를 적용했습니다.`,
      );
      return outcome.profile;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  }

  function toggleDemo() {
    if (busyRef.current || profileDraft) return;
    setDemo(!demo);
    setConnected(false);
    setDevices(demo ? [] : demoDevices.map((d) => ({ ...d })));
    setError("");
    setMessage("");
    setFailures({});
    setFilter("");
    setExpanded(null);
    setMouseTab("lighting");
    setMouseStatus(null);
  }

  return (
    <div className="app-shell">
      <Titlebar busy={!!busy} onError={setError} />
      <main>
        <header className="app-header">
          <div>
            <h1>내 PC의 조명</h1>
            <p className="page-description">빛이 필요할 때, 한 번의 클릭.</p>
          </div>
          <span className={`connection ${available ? "ready" : ""}`}>
            <span />
            {demo ? "데모" : connected ? "연결됨" : "연결 대기"}
          </span>
        </header>

        {demo && (
          <div className="demo-banner">
            <span>데모 미리보기 · 실제 조명은 바뀌지 않습니다.</span>
            <button onClick={toggleDemo} disabled={!!busy || !!profileDraft}>
              데모 종료
            </button>
          </div>
        )}

        <section className="master" aria-labelledby="master-heading">
          <div className="master-heading">
            <h2 id="master-heading">모든 조명</h2>
            <span>제어 가능한 장치를 한 번에</span>
          </div>
          <div className="master-actions">
            <button
              className="master-button on"
              disabled={!available || !supported.length || !!busy}
              onClick={() => power(true, supported)}
            >
              <Power size={21} />
              <span>{profileDraft ? "프로파일 전체 ON" : "전체 켜기"}</span>
              <small>ON</small>
            </button>
            <button
              className="master-button off"
              disabled={!available || !supported.length || !!busy}
              onClick={() => power(false, supported)}
            >
              <Power size={21} />
              <span>{profileDraft ? "프로파일 전체 OFF" : "전체 끄기"}</span>
              <small>OFF</small>
            </button>
          </div>
          <div className="master-meta">
            <p>
              {available
                ? `${supported.length}개 제어 가능 · ${devices.length - supported.length}개 확인 필요`
                : "장치를 확인하면 제어할 수 있습니다"}
            </p>
            <span className="spectrum" aria-hidden="true" />
          </div>
        </section>
        <p className="verification-note">
          <Info size={12} />
          최근 명령을 표시합니다 · 실제 발광 변화는 미확인
        </p>

        <div className="feedback" aria-live="polite" aria-atomic="true">
          {busy ? (
            <p>
              <RefreshCw size={16} className="spinning" />
              {busy}
            </p>
          ) : message ? (
            <p>
              <Check size={16} />
              {message}
            </p>
          ) : null}
        </div>
        {error && (
          <div role="alert" className="error">
            <AlertCircle size={19} />
            <span>{error}</span>
          </div>
        )}

        <Profiles
          key={String(demo)}
          demo={demo}
          busy={!!busy}
          draft={profileDraft}
          onEdit={editProfile}
          onNew={newProfile}
          onApply={applyProfile}
          onSaving={setProfileSaving}
        />
        <section
          className="devices"
          aria-labelledby="devices-heading"
          aria-busy={!!busy}
        >
          <div className="section-heading">
            <h2 id="devices-heading">
              {profileDraft ? "프로파일 장치 설정" : "장치"}{" "}
              {available && <span>{devices.length}</span>}
            </h2>
            <button
              className="text-button"
              onClick={scan}
              disabled={!!busy || !!profileDraft}
            >
              <RefreshCw size={15} />
              {connected || demo ? "새로고침" : "장치 확인"}
            </button>
          </div>
          {!available ? (
            <div className="empty">
              <Unplug size={27} strokeWidth={1.5} />
              <h3>장치를 연결해 볼까요?</h3>
              <p>
                키보드부터 PC 내부 조명까지 함께 제어합니다.
                <br />
                메모리 제어는 관리자 권한으로 실행하세요.
              </p>
              <button className="primary" onClick={scan} disabled={!!busy}>
                장치 확인하기
                <ArrowUpRight size={15} />
              </button>
              <button
                className="text-button demo-link"
                onClick={toggleDemo}
                disabled={!!busy}
              >
                데모로 둘러보기
              </button>
            </div>
          ) : devices.length === 0 ? (
            <div className="empty">
              <Lightbulb size={32} />
              <h3>감지된 RGB 장치가 없습니다</h3>
              <p>
                USB 연결과 그래픽 드라이버를 확인한 뒤<br />
                목록을 새로고침하세요.
              </p>
            </div>
          ) : (
            <>
              {devices.length > 4 && (
                <input
                  className="search"
                  aria-label="장치 검색"
                  placeholder="장치 이름 검색"
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                />
              )}
              <ul className="device-list">
                {devices.map((device) => {
                  const { Icon, label } = deviceKind(device.kind);
                  return (
                    <li
                      key={`${demo}:${!!profileDraft}:${profileRevision}:${device.id}:${device.key}`}
                      hidden={
                        !device.name
                          .toLowerCase()
                          .includes(filter.toLowerCase())
                      }
                      className={!device.supported ? "device-pending" : ""}
                    >
                      <div className="device-row">
                        <Icon
                          size={24}
                          strokeWidth={1.5}
                          className="device-icon"
                        />
                        <div className="device-copy">
                          <h3>
                            <button
                              className="device-name-button"
                              aria-expanded={expanded === device.key}
                              aria-controls={`detail-${device.id}`}
                              onClick={() =>
                                setExpanded(
                                  expanded === device.key ? null : device.key,
                                )
                              }
                            >
                              {device.name
                                .replace("CORSAIR ", "")
                                .replace(" · RGB 헤더", "")}
                            </button>
                          </h3>
                          <p>
                            {device.lastLighting && (
                              <span
                                className="device-swatch"
                                aria-hidden="true"
                                style={{
                                  background:
                                    device.lastLighting.effect === "spectrum"
                                      ? "var(--warm)"
                                      : colorHex(device.lastLighting.color),
                                }}
                              />
                            )}
                            {label} <span className="meta-separator">/</span>{" "}
                            {profileDraft
                              ? `저장할 설정 · ${device.lastRequested ? "ON" : "OFF"}`
                              : !device.supported
                                ? "연결·권한 확인 필요"
                                : device.lastRequested === null
                                  ? "명령 대기"
                                  : `최근 명령 ${device.lastRequested ? (device.lastLighting ? effectNames[device.lastLighting.effect] : "ON") : "OFF"}`}
                          </p>
                        </div>
                        <div
                          className="device-actions"
                          role="group"
                          aria-label={`${device.name} RGB 제어`}
                        >
                          <button
                            className="device-on"
                            aria-label={`${device.name} 켜기`}
                            aria-pressed={device.lastRequested === true}
                            onClick={() => power(true, [device])}
                            disabled={!!busy || !device.supported}
                          >
                            ON
                          </button>
                          <button
                            aria-label={`${device.name} 끄기`}
                            aria-pressed={device.lastRequested === false}
                            onClick={() => power(false, [device])}
                            disabled={!!busy || !device.supported}
                          >
                            OFF
                          </button>
                        </div>
                        <button
                          className="device-info"
                          aria-label={`${device.name} 설정 펼치기`}
                          aria-expanded={expanded === device.key}
                          aria-controls={`detail-${device.id}`}
                          onClick={() =>
                            setExpanded(
                              expanded === device.key ? null : device.key,
                            )
                          }
                        >
                          <ChevronDown size={16} />
                        </button>
                      </div>
                      <div
                        className="device-expanded"
                        id={`detail-${device.id}`}
                        hidden={expanded !== device.key}
                      >
                        {device.supported && device.effects.length > 0 && (
                          <>
                            {device.id === 0 && (
                              <div
                                className="device-setting-tabs"
                                role="group"
                                aria-label="마우스 설정 종류"
                              >
                                <button
                                  aria-pressed={mouseTab === "lighting"}
                                  onClick={() => setMouseTab("lighting")}
                                >
                                  조명
                                </button>
                                <button
                                  aria-pressed={mouseTab === "dpi"}
                                  onClick={() => setMouseTab("dpi")}
                                >
                                  DPI · 표시등
                                </button>
                              </div>
                            )}
                            <div
                              hidden={
                                device.id === 0 && mouseTab !== "lighting"
                              }
                            >
                              <LightingPanel
                                device={device}
                                busy={!!busy}
                                onApply={applyLighting}
                                value={
                                  profileDraft?.devices.find(
                                    (e) => e.id === device.id,
                                  )?.lighting
                                }
                                onChange={
                                  profileDraft
                                    ? (value) =>
                                        editProfileLighting(device.id, value)
                                    : undefined
                                }
                              />
                            </div>
                            {device.id === 0 && (
                              <div hidden={mouseTab !== "dpi"}>
                                {profileDraft && (
                                  <label className="profile-dpi-option">
                                    <input
                                      type="checkbox"
                                      checked={!!profileDraft.mouseDpi}
                                      onChange={(e) =>
                                        setProfileDraft(
                                          (p) =>
                                            p && {
                                              ...p,
                                              mouseDpi: e.target.checked
                                                ? structuredClone(defaultDpi)
                                                : null,
                                            },
                                        )
                                      }
                                    />
                                    DPI·표시등도 프로파일에 포함
                                  </label>
                                )}
                                {(!profileDraft || profileDraft.mouseDpi) && (
                                  <DpiPanel
                                    active={
                                      expanded === device.key &&
                                      mouseTab === "dpi"
                                    }
                                    busy={!!busy}
                                    status={mouseStatus}
                                    onLoad={loadDpi}
                                    onApply={applyDpi}
                                    value={profileDraft?.mouseDpi ?? undefined}
                                    onChange={
                                      profileDraft
                                        ? (value) =>
                                            setProfileDraft(
                                              (p) =>
                                                p && { ...p, mouseDpi: value },
                                            )
                                        : undefined
                                    }
                                  />
                                )}
                                {profileDraft && !profileDraft.mouseDpi && (
                                  <p className="profile-hint">
                                    현재 DPI 설정을 유지합니다. 포함을 켜면 기본
                                    5단계 값에서 편집할 수 있습니다.
                                  </p>
                                )}
                              </div>
                            )}
                          </>
                        )}
                        <p className="device-detail">
                          {device.detail || device.mode}
                        </p>
                      </div>
                      {failures[device.id] && (
                        <p className="device-error">{failures[device.id]}</p>
                      )}
                    </li>
                  );
                })}
              </ul>
              {filter &&
                !devices.some((d) =>
                  d.name.toLowerCase().includes(filter.toLowerCase()),
                ) && <p className="no-results">검색 결과가 없습니다.</p>}
            </>
          )}
        </section>

        <MonitorPanel key={`monitors-${demo}`} demo={demo} />

        <section className="startup-setting" aria-label="앱 시작 설정">
          <label>
            <input
              type="checkbox"
              checked={startup.enabled}
              disabled={!isTauri() || !startup.ready || startupBusy}
              onChange={(e) => void toggleStartup(e.target.checked)}
            />
            Windows 로그인 시 트레이에서 시작
          </label>
          <p>
            {isTauri() && (!startup.ready || startupBusy)
              ? "자동 시작 설정을 확인하고 있습니다…"
              : "창을 닫으면 트레이로 숨깁니다. 완전히 끝내려면 트레이 메뉴에서 ‘종료’를 선택하세요."}
          </p>
          {startup.error && (
            <p role="alert" className="startup-error">
              {startup.error}
            </p>
          )}
        </section>
        <footer>
          <button
            className="text-button"
            aria-expanded={settings}
            aria-controls="connection-settings"
            onClick={() => setSettings(!settings)}
          >
            <Settings2 size={14} />
            제어 안내
          </button>
          <span>
            로컬 장치 제어 <span className="version">v0.1.0</span>
          </span>
        </footer>
        {settings && (
          <section
            className="settings"
            id="connection-settings"
            aria-label="제어 안내"
          >
            <p>
              장치를 펼쳐 단색·숨쉬기·색상 순환을 설정할 수 있습니다. 이번
              실행에서 적용한 효과는 켜기 버튼으로 다시 켤 수 있습니다. 끄기는
              RGB 조명만 끄며 장치 전원이나 팬 회전은 바꾸지 않습니다.
            </p>
            <p>
              HARPOON과 키보드는 USB 유선 연결을 사용하세요. iCUE, GANSS
              드라이버나 Mystic Light가 동시에 조명을 변경하면 설정이 덮어써질
              수 있습니다. 해당 앱의 조명 제어를 종료한 뒤 다시 시도하세요.
            </p>
            <p>
              메인보드 RGB 헤더에 연결된 팬은 메인보드 버튼으로 함께 제어합니다.
              별도 허브의 조명은 연결 방식에 따라 다릅니다. 메모리 두 개는 함께
              제어하며, 앱에 내장된 메모리 드라이버와 관리자 권한이 필요합니다.
            </p>
          </section>
        )}
      </main>
    </div>
  );
}
