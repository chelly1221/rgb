import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Plus, Pencil, Play, Save, X } from "lucide-react";
import type { Lighting } from "./LightingPanel";
import type { DpiProfile } from "./DpiPanel";

export type ProfileEntry = {
  id: number;
  key: string;
  enabled: boolean;
  lighting: Lighting;
};
export type SceneProfile = {
  id: string;
  name: string;
  devices: ProfileEntry[];
  mouseDpi: DpiProfile | null;
};
export const deviceKeys = [
  "corsair-harpoon-wired-1b1c-1b5e",
  "palit-rtx3070-10de-2484-1569-2484",
  "msi-b860m-mortar-7e40",
  "ganss-gs3087t-05ac-0256",
  "corsair-cmh128gx5m2b6400c42-pair",
];
export const defaultDpi: DpiProfile = {
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
};
export const defaultLighting: Lighting = {
  effect: "static",
  color: [255, 164, 211],
  brightness: 70,
  speed: 2,
};

export default function Profiles({
  demo,
  busy,
  draft,
  onEdit,
  onNew,
  onApply,
  onSaving,
}: {
  demo: boolean;
  busy: boolean;
  draft: SceneProfile | null;
  onEdit: (profile: SceneProfile | null) => void;
  onNew: () => SceneProfile;
  onApply: (profile: SceneProfile) => Promise<void>;
  onSaving: (saving: boolean) => void;
}) {
  const [profiles, setProfiles] = useState<SceneProfile[]>([]);
  const [selected, setSelected] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const savingRef = useRef(false);
  const native = isTauri() && !demo;
  const current = profiles.find((p) => p.id === selected);
  useEffect(() => {
    let disposed = false;
    const load = async () => {
      try {
        const items: SceneProfile[] = native
          ? await invoke("list_profiles")
          : JSON.parse(
              localStorage.getItem("rgb-switch.demo-profiles.v1") ?? "[]",
            );
        if (
          !Array.isArray(items) ||
          items.length > 32 ||
          items.some(
            (p) =>
              !p ||
              typeof p.id !== "string" ||
              typeof p.name !== "string" ||
              !Array.isArray(p.devices) ||
              p.devices.length !== 5,
          )
        )
          throw new Error("프로파일 목록 형식이 올바르지 않습니다.");
        if (!disposed) {
          setProfiles(items);
          setSelected(items[0]?.id ?? "");
        }
      } catch (e) {
        if (!disposed) setError(String(e));
      } finally {
        if (!disposed) setLoading(false);
      }
    };
    void load();
    return () => {
      disposed = true;
    };
  }, [native]);
  async function save() {
    if (!draft || savingRef.current) return;
    const profile = { ...draft, name: draft.name.trim() };
    if (!profile.name || profile.name.length > 40) {
      setError("프로파일 이름을 1~40자로 입력하세요.");
      return;
    }
    if (
      profile.mouseDpi &&
      profile.mouseDpi.values.some(
        (v) => !Number.isInteger(v) || v < 200 || v > 10000 || v % 200 !== 0,
      )
    ) {
      setError("DPI를 200~10000 사이의 200 단위로 입력하세요.");
      return;
    }
    if (
      profiles.some(
        (p) =>
          p.id !== profile.id &&
          p.name.toLowerCase() === profile.name.toLowerCase(),
      )
    ) {
      setError("같은 이름이 있습니다. 다른 이름을 입력하세요.");
      return;
    }
    savingRef.current = true;
    onSaving(true);
    setSaving(true);
    setError("");
    try {
      let next: SceneProfile[];
      if (native) next = await invoke("save_profile", { profile });
      else {
        next = profiles.some((p) => p.id === profile.id)
          ? profiles.map((p) => (p.id === profile.id ? profile : p))
          : [...profiles, profile];
        if (next.length > 32)
          throw new Error("프로파일은 최대 32개까지 저장할 수 있습니다.");
        localStorage.setItem(
          "rgb-switch.demo-profiles.v1",
          JSON.stringify(next),
        );
      }
      setProfiles(next);
      setSelected(profile.id);
      onEdit(null);
      setNotice(`‘${profile.name}’ 저장 완료.`);
      await onApply(profile);
    } catch (e) {
      setError(String(e));
    } finally {
      savingRef.current = false;
      onSaving(false);
      setSaving(false);
    }
  }
  return (
    <section
      className={`profiles ${draft ? "profiles-editing" : ""}`}
      aria-labelledby="profiles-heading"
    >
      <div className="section-heading">
        <h2 id="profiles-heading">전체 프로파일</h2>
        <span className="profile-count">{profiles.length} / 32</span>
      </div>
      {draft ? (
        <>
          <label className="profile-name">
            프로파일 이름
            <input
              autoFocus
              maxLength={40}
              value={draft.name}
              disabled={saving}
              placeholder="예: 작업, 게임, 취침"
              onChange={(e) => onEdit({ ...draft, name: e.target.value })}
            />
          </label>
          <p className="profile-hint">
            아래 장치를 펼쳐 ON/OFF·색상·효과를 편집하세요. 저장하면 전체 장치에
            바로 적용됩니다.
          </p>
          <div className="profile-buttons">
            <button
              className="primary"
              disabled={
                saving || busy || !draft.name.trim() || (!native && !demo)
              }
              onClick={() => void save()}
            >
              <Save size={15} />
              {saving ? "저장·적용 중…" : "저장·적용"}
            </button>
            <button
              className="text-button"
              disabled={saving || busy}
              onClick={() => {
                onEdit(null);
                setError("");
                setNotice("편집을 취소했습니다.");
              }}
            >
              <X size={15} />
              취소
            </button>
          </div>
        </>
      ) : (
        <>
          <div className="profile-toolbar">
            <select
              aria-label="저장된 프로파일"
              value={selected}
              disabled={loading || busy || !profiles.length}
              onChange={(e) => {
                setSelected(e.target.value);
                setNotice("");
              }}
            >
              {!profiles.length && (
                <option value="">
                  {loading ? "불러오는 중…" : "저장된 프로파일 없음"}
                </option>
              )}
              {profiles.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <button
              className="primary"
              disabled={!current || busy || (!native && !demo)}
              onClick={() => current && void onApply(structuredClone(current))}
            >
              <Play size={15} />
              일괄 적용
            </button>
          </div>
          <div className="profile-buttons">
            <button
              className="text-button"
              disabled={loading || busy || !!error || profiles.length >= 32}
              onClick={() => {
                setNotice("");
                setError("");
                onEdit(onNew());
              }}
            >
              <Plus size={15} />새 프로파일
            </button>
            <button
              className="text-button"
              disabled={!current || busy}
              onClick={() => {
                setNotice("");
                setError("");
                onEdit(structuredClone(current!));
              }}
            >
              <Pencil size={15} />
              수정
            </button>
          </div>
          {!profiles.length && !loading && (
            <p className="profile-hint">
              5개 장치의 설정을 한 번에 저장합니다. 새 프로파일은 최근 적용값을
              사용하며, 기록이 없으면 분홍색 70%로 시작합니다.
            </p>
          )}
        </>
      )}
      {!native && (
        <p className="profile-hint">
          데모 프로파일은 이 브라우저에만 저장됩니다.
        </p>
      )}
      {notice && (
        <p className="profile-notice" role="status">
          {notice}
        </p>
      )}
      {error && (
        <p className="profile-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
