import { type JSX } from "@solidjs/web";

const paths = {
  pause: "M8 5v14M16 5v14",
  play: "m9 5 11 7-11 7Z",
  key: "M14 8a5 5 0 1 1-10 0 5 5 0 0 1 10 0Zm-1 4 8 8m-4-4 3-3m-1 5 3-3",
  coin: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20ZM7 14V8l3 2h4l3-2v6c-2 3-8 3-10 0ZM9 13h.01M15 13h.01",
  pack: "M5 5h4v14H5Zm10 0h4v14h-4ZM9 8h6v8H9M7 22v-1m10 1v-1M10 3l2-2 2 2",
  close: "m6 6 12 12M6 18 18 6",
  home: "m3 11 9-8 9 8M6 10v11h12V10M10 21v-6h4v6",
  restart: "M4 10a8 8 0 1 1 1 8M4 4v6h6",
  skull: "M6 16a9 9 0 1 1 12 0v5H6ZM8 10h.01M16 10h.01M10 21v-3m4 3v-3M11 14h2",
  food: "M3 10h18l-3 9H6Zm2 0a3 3 0 0 1 5-3 3 3 0 0 1 5 0 3 3 0 0 1 5 3M9 14h6",
  case: "M3 7h18v14H3ZM8 7V3h8v4M7 7v14M17 7v14M10 13h4",
  talk: "M21 11a9 8 0 0 1-12 8l-6 3 2-6a8 8 0 1 1 16-5ZM8 11h.01M12 11h.01M16 11h.01",
  compass: "M12 2v3m0 14v3M2 12h3m14 0h3M8 16l3-7 5-1-3 7Z",
  trophy: "M8 3h8v8a4 4 0 0 1-8 0ZM8 5H3v3a5 5 0 0 0 5 5m8-8h5v3a5 5 0 0 1-5 5M12 15v6m-4 0h8",
  arrow: "M4 12h16m-6-6 6 6-6 6",
} as const;
export type IconName = keyof typeof paths | "help";

export function Icon(props: { name: IconName; size?: number; xstyle?: JSX.CSSProperties }) {
  return (
    <svg
      width={props.size ?? 24}
      height={props.size ?? 24}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
      style={props.xstyle}
    >
      {props.name === "help" ? (
        <>
          <circle cx="12" cy="12" r="10" />
          <path d="M9 8a3 3 0 0 1 6 0c0 3-3 2-3 5m0 4h.01" />
        </>
      ) : (
        <path d={paths[props.name]} />
      )}
    </svg>
  );
}
