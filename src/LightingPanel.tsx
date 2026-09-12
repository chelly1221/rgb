import { useState, type CSSProperties } from "react";
import { Check } from "lucide-react";

export type Effect = "static" | "breathe" | "spectrum";
export type Lighting = {
  effect: Effect;
  color: [number, number, number];
  brightness: number;
  speed: number;
};
export const effectNames: Record<Effect, string> = {
  static: "단색",
  breathe: "숨쉬기",
  spectrum: "색상 순환",
};
type Target = {
  id: number;
  key: string;
  name: string;
  supported: boolean;
  effects: Effect[];
  lastLighting?: Lighting | null;
};
export const colorHex = (rgb: number[]) =>
  "#" + rgb.map((v) => v.toString(16).padStart(2, "0")).join("");
export const colorRgb = (hex: string) =>
  [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [
    number,
    number,
    number,
  ];
const defaultLighting: Lighting = {
  effect: "static",
  color: [255, 164, 211],
  brightness: 70,
  speed: 2,
};
const swatches = [
  ["분홍", "#ffa4d3"],
  ["다홍", "#ff7c6d"],
  ["주황", "#ffab66"],
  ["노랑", "#ffdc88"],
  ["파랑", "#8aafff"],
  ["흰색", "#ffffff"],
];
export default function LightingPanel({
  device,
  busy,
  onApply,
}: {
  device: Target;
  busy: boolean;
  onApply: (settings: Lighting, ids: number[]) => Promise<void>;
}) {
  const [draft, setDraft] = useState<Lighting | null>(null);
  const settings = draft ?? device.lastLighting ?? defaultLighting;
  const effect = device.effects.includes(settings.effect)
    ? settings.effect
    : "static";
  const { brightness, speed } = settings;
  const color = colorHex(settings.color);
  const update = (values: Partial<Lighting>) =>
    setDraft({ ...settings, effect, ...values });
  return (
    <section
      className="device-lighting"
      aria-label={`${device.name} 조명 설정`}
    >
      <div className="lighting-controls">
        <fieldset disabled={busy || !device.supported}>
          <legend className="sr-only">조명 설정</legend>
          <div className="lighting-selects device-effect-select">
            <label>
              효과
              <select
                value={effect}
                onChange={(e) => update({ effect: e.target.value as Effect })}
              >
                {device.effects.map((key) => (
                  <option key={key} value={key}>
                    {effectNames[key]}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="lighting-color-row">
            <span className="lighting-label">색상</span>
            <div
              className="color-swatches"
              role="group"
              aria-label="색상 프리셋"
            >
              {swatches.map(([name, hex]) => (
                <button
                  key={hex}
                  type="button"
                  title={name}
                  aria-label={name}
                  aria-pressed={color === hex}
                  disabled={effect === "spectrum"}
                  style={{ "--swatch": hex } as CSSProperties}
                  onClick={() => update({ color: colorRgb(hex) })}
                >
                  {color === hex && <Check size={14} />}
                </button>
              ))}
            </div>
            <label
              className={`custom-color ${effect === "spectrum" ? "inactive" : ""}`}
            >
              <input
                type="color"
                value={color}
                onChange={(e) => update({ color: colorRgb(e.target.value) })}
                aria-label="직접 색상 선택"
                disabled={effect === "spectrum"}
              />
              <span>직접 선택</span>
            </label>
          </div>
          <div className="lighting-ranges">
            <label htmlFor={`led-brightness-${device.id}`}>
              <span>
                밝기<output aria-hidden="true">{brightness}%</output>
              </span>
              <input
                id={`led-brightness-${device.id}`}
                type="range"
                aria-valuetext={`${brightness}%`}
                min="0"
                max="100"
                step="1"
                value={brightness}
                onChange={(e) => update({ brightness: Number(e.target.value) })}
              />
            </label>
            <label
              htmlFor={`led-speed-${device.id}`}
              className={effect === "static" ? "inactive" : ""}
            >
              <span>
                속도
                <output aria-hidden="true">
                  {["느리게", "보통", "빠르게"][speed - 1]}
                </output>
              </span>
              <input
                id={`led-speed-${device.id}`}
                type="range"
                aria-valuetext={["느리게", "보통", "빠르게"][speed - 1]}
                min="1"
                max="3"
                step="1"
                value={speed}
                disabled={effect === "static"}
                onChange={(e) => update({ speed: Number(e.target.value) })}
              />
            </label>
          </div>
          <div className="lighting-bottom">
            <p>이 장치에만 적용합니다.</p>
            <button
              className="primary"
              disabled={!device.effects.length}
              onClick={() => {
                const rgb = [1, 3, 5].map((i) =>
                  parseInt(color.slice(i, i + 2), 16),
                ) as [number, number, number];
                void onApply({ effect, color: rgb, brightness, speed }, [
                  device.id,
                ]);
              }}
            >
              조명 적용
            </button>
          </div>
        </fieldset>
        {device.id === 0 && (
          <p className="lighting-note">
            로고는 앱 실행 중 단색을 유지합니다. DPI 표시등은 DPI 설정에서 바꿀
            수 있습니다.
          </p>
        )}
        {device.id === 2 && (
          <p className="lighting-note">
            메인보드 헤더에 연결된 팬 조명을 함께 변경합니다.
          </p>
        )}
        {device.id === 4 && (
          <p className="lighting-note">메모리 두 개에 함께 적용합니다.</p>
        )}
      </div>
    </section>
  );
}
