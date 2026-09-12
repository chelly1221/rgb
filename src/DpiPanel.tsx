import { useEffect, useRef, useState } from "react";
import { colorHex, colorRgb } from "./LightingPanel";

export type DpiProfile = {
  values: number[];
  colors: [number, number, number][];
  mask: number;
  index: number;
};
export type MouseStatus = {
  active: boolean;
  profile: DpiProfile | null;
  error: string | null;
};
export default function DpiPanel({
  active,
  busy,
  status,
  onLoad,
  onApply,
}: {
  active: boolean;
  busy: boolean;
  status: MouseStatus | null;
  onLoad: () => Promise<DpiProfile | null>;
  onApply: (profile: DpiProfile) => Promise<DpiProfile | null>;
}) {
  const attempted = useRef(false);
  const [profile, setProfile] = useState<DpiProfile | null>(null);
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);
  const valid =
    !!profile &&
    profile.values.every(
      (v) => Number.isInteger(v) && v >= 200 && v <= 10000 && v % 200 === 0,
    );
  const edit = (next: DpiProfile) => {
    setProfile(next);
    setDirty(true);
    setSaved(false);
  };
  const load = async () => {
    const next = await onLoad();
    if (next) {
      setProfile(next);
      setDirty(false);
      setSaved(false);
    }
  };
  useEffect(() => {
    if (active && !busy && !profile && !attempted.current) {
      attempted.current = true;
      void load();
    }
  });
  return (
    <section className="device-dpi" aria-label="DPI 설정">
      <div className="dpi-current">
        {status?.active && status.profile
          ? `현재 ${status.profile.values[status.profile.index].toLocaleString()} DPI`
          : "5단계 감도와 표시등"}
      </div>
      <div className="lighting-controls">
        {!profile ? (
          <div className="dpi-empty">
            <p className="lighting-note">
              {busy
                ? "DPI 설정을 읽고 있습니다…"
                : "마우스에서 DPI 설정을 가져오세요."}
            </p>
            <button
              className="text-button"
              disabled={busy}
              onClick={() => void load()}
            >
              다시 불러오기
            </button>
          </div>
        ) : (
          <>
            <p className="dpi-description">
              감도는 200~10000 DPI, 200 단위로 지정합니다. 표시등 색상은 로고와
              별도로 적용됩니다.
            </p>
            <fieldset disabled={busy}>
              <legend className="sr-only">DPI 5단계 설정</legend>
              <div className="dpi-row dpi-labels" aria-hidden="true">
                <span>사용</span>
                <span>단계</span>
                <span>감도 · DPI</span>
                <span>표시등</span>
                <span>적용 단계</span>
              </div>
              {profile.values.map((value, i) => {
                const enabled = !!(profile.mask & (1 << i));
                return (
                  <div
                    className={`dpi-row ${!enabled ? "dpi-disabled" : ""}`}
                    key={i}
                  >
                    <input
                      type="checkbox"
                      aria-label={`DPI ${i + 1}단계 사용`}
                      checked={enabled}
                      disabled={enabled && profile.mask === 1 << i}
                      onChange={() => {
                        const mask = profile.mask ^ (1 << i);
                        edit({
                          ...profile,
                          mask,
                          index:
                            mask & (1 << profile.index)
                              ? profile.index
                              : profile.values.findIndex(
                                  (_, j) => !!(mask & (1 << j)),
                                ),
                        });
                      }}
                    />
                    <span className="dpi-stage">
                      {i + 1}
                      <span className="sr-only">단계</span>
                    </span>
                    <input
                      type="number"
                      min={200}
                      max={10000}
                      step={200}
                      value={value || ""}
                      aria-label={`DPI ${i + 1}단계 감도`}
                      aria-invalid={
                        value < 200 || value > 10000 || value % 200 !== 0
                      }
                      onChange={(e) =>
                        edit({
                          ...profile,
                          values: profile.values.map((v, j) =>
                            j === i ? Number(e.target.value) : v,
                          ),
                        })
                      }
                    />
                    <input
                      type="color"
                      value={colorHex(profile.colors[i])}
                      aria-label={`DPI ${i + 1}단계 표시등 색상`}
                      onChange={(e) =>
                        edit({
                          ...profile,
                          colors: profile.colors.map((c, j) =>
                            j === i ? colorRgb(e.target.value) : c,
                          ),
                        })
                      }
                    />
                    <input
                      type="radio"
                      name="dpi-stage"
                      checked={profile.index === i}
                      disabled={!enabled}
                      aria-label={`DPI ${i + 1}단계를 현재 단계로 적용`}
                      onChange={() => edit({ ...profile, index: i })}
                    />
                  </div>
                );
              })}
              {!valid && (
                <p className="dpi-validation" role="alert">
                  각 DPI를 200~10000 사이의 200 단위로 입력하세요.
                </p>
              )}
              <div className="lighting-bottom">
                <p aria-live="polite">
                  {dirty
                    ? "변경한 설정을 적용해 주세요."
                    : saved
                      ? "앱 설정에 저장하고 적용했습니다."
                      : "버튼을 누르면 활성 단계가 순서대로 바뀝니다."}
                </p>
                <button
                  className="primary"
                  disabled={!valid}
                  onClick={async () => {
                    const next = await onApply(profile);
                    if (next) {
                      setProfile(next);
                      setDirty(false);
                      setSaved(true);
                    }
                  }}
                >
                  DPI 저장·적용
                </button>
              </div>
            </fieldset>
            <p className="lighting-note">
              앱 실행 중 적용됩니다. 종료하면 마우스에 원래 저장된 DPI와 조명
              모드로 돌아갑니다. 다시 실행한 뒤 조명 또는 DPI를 적용하면 저장한
              설정을 사용합니다.
            </p>
          </>
        )}
      </div>
    </section>
  );
}
