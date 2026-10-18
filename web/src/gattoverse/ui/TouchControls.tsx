import * as stylex from "@stylexjs/stylex";
import { createSignal } from "solid-js";
import { colors } from "../../design-system/tokens.stylex";
import { Icon } from "./Icon";

export function TouchControls(props: {
  flight: boolean;
  active: boolean;
  steer: (x: number, y: number) => void;
  boost: (pressed: boolean) => void;
}) {
  const [stick, setStick] = createSignal({ x: 0, y: 0 });
  let pad: HTMLDivElement | undefined;
  const reset = () => {
    setStick({ x: 0, y: 0 });
    props.steer(0, 0);
  };
  const move = (event: PointerEvent) => {
    if (!props.active || !pad?.hasPointerCapture(event.pointerId)) return;
    const bounds = pad.getBoundingClientRect();
    const dx = event.clientX - bounds.x - bounds.width / 2,
      dy = event.clientY - bounds.y - bounds.height / 2;
    const length = Math.hypot(dx, dy),
      radius = 38;
    const amount = Math.min(length, radius),
      x = length > 5 ? (dx / length) * amount : 0,
      y = length > 5 ? (dy / length) * amount : 0;
    setStick({ x, y });
    props.steer(x / radius, y / radius);
  };
  return (
    <div {...stylex.attrs(styles.controls)}>
      <div
        ref={(el) => {
          pad = el;
        }}
        {...stylex.attrs(styles.pad)}
        role="group"
        aria-label="Movement thumbstick"
        tabindex="0"
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          pad?.setPointerCapture(event.pointerId);
          move(event);
        }}
        onPointerMove={move}
        onPointerUp={reset}
        onPointerCancel={reset}
        onLostPointerCapture={reset}
        onBlur={reset}
        onKeyDown={(event) => {
          const vectors: Record<string, [number, number]> = {
            ArrowLeft: [-1, 0],
            ArrowRight: [1, 0],
            ArrowUp: [0, -1],
            ArrowDown: [0, 1],
          };
          const vector = vectors[event.key];
          if (vector) {
            event.preventDefault();
            props.steer(vector[0], vector[1]);
          }
        }}
        onKeyUp={reset}
      >
        <div {...stylex.attrs(styles.center)} />
        <div
          {...stylex.attrs(styles.thumb)}
          style={{ translate: `${stick().x}px ${stick().y}px` }}
        />
      </div>
      {props.flight && (
        <button
          type="button"
          aria-label="Hold to thrust"
          {...stylex.attrs(styles.boost)}
          disabled={!props.active}
          onPointerDown={(event) => {
            if (event.currentTarget instanceof HTMLButtonElement)
              event.currentTarget.setPointerCapture(event.pointerId);
            props.boost(true);
          }}
          onPointerUp={() => props.boost(false)}
          onPointerCancel={() => props.boost(false)}
          onLostPointerCapture={() => props.boost(false)}
          onBlur={() => props.boost(false)}
          onKeyDown={(event) => {
            if (event.key === " " || event.key === "Enter") {
              event.preventDefault();
              props.boost(true);
            }
          }}
          onKeyUp={() => props.boost(false)}
        >
          <Icon name="pack" size={33} />
        </button>
      )}
    </div>
  );
}

const styles = stylex.create({
  controls: {
    display: { default: "none", "@media (pointer: coarse), (width <= 760px)": "flex" },
    position: "absolute",
    insetInline: 24,
    insetBlockEnd: "max(24px, env(safe-area-inset-bottom))",
    justifyContent: "space-between",
    alignItems: "center",
    pointerEvents: "none",
    zIndex: 4,
  },
  pad: {
    position: "relative",
    width: 118,
    height: 118,
    borderRadius: "50%",
    backgroundColor: "#10203559",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#e3e1e033",
    pointerEvents: "auto",
    touchAction: "none",
    userSelect: "none",
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
    outlineOffset: 4,
  },
  center: {
    position: "absolute",
    inset: 17,
    borderRadius: "50%",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#e3e1e01c",
  },
  thumb: {
    position: "absolute",
    inset: 34,
    borderRadius: "50%",
    backgroundImage: "radial-gradient(circle at 35% 25%, #e0dfc877, #8997a733)",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#e3e1e077",
    boxShadow: "0 4px 15px #0004",
    pointerEvents: "none",
  },
  boost: {
    display: "grid",
    placeItems: "center",
    width: 74,
    height: 74,
    borderRadius: "50%",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#a8f0ce88",
    backgroundColor: "#162c35aa",
    color: colors.gameMint,
    pointerEvents: "auto",
    touchAction: "none",
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
    outlineOffset: 4,
  },
});
