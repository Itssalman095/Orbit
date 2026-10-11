import { useMemo, useState } from "react";

export interface AppIconProps {
  /** Display name, used for the letter-avatar fallback. */
  name: string;
  /** Icon image (usually a data: URL from the Rust extractor). */
  src?: string | null;
  /** Icon size in CSS pixels. */
  size: number;
}

/** Deterministic pastel hue for fallback avatars. */
function hueFromString(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) >>> 0;
  }
  return h % 360;
}

/**
 * Renders an application icon with a graceful letter-avatar fallback.
 */
export function AppIcon({ name, src, size }: AppIconProps) {
  const [failed, setFailed] = useState(false);
  const hue = useMemo(() => hueFromString(name || "?"), [name]);
  const letter = useMemo(() => {
    const m = name.match(/[a-zA-Z0-9]/);
    return m ? m[0].toUpperCase() : "?";
  }, [name]);

  const showImage = Boolean(src) && !failed;

  return (
    <div className="app-icon" style={{ width: size, height: size }}>
      {showImage ? (
        <img
          key={src ?? undefined}
          src={src ?? undefined}
          alt=""
          draggable={false}
          onError={() => setFailed(true)}
        />
      ) : (
        <div
          className="app-icon-fallback"
          style={{
            background: `linear-gradient(135deg, hsl(${hue} 55% 42%), hsl(${(hue + 40) % 360} 55% 32%))`,
            fontSize: Math.round(size * 0.42),
          }}
        >
          {letter}
        </div>
      )}
    </div>
  );
}
