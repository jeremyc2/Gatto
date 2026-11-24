export const themeOptions = ["system", "light", "dark"] as const;
export type ThemePreference = (typeof themeOptions)[number];

const storageKey = "gatto.theme";
const selector = 'meta[name="color-scheme"]';

// Runs just after the meta tag, before the browser paints the page.
export const applyStoredTheme = `try{const t=localStorage.getItem("${storageKey}");if(t==="light"||t==="dark")document.currentScript.previousElementSibling.content=t}catch{}`;

function preference(value: string | null | undefined): ThemePreference {
  return value === "light" || value === "dark" ? value : "system";
}

export function themeMeta(): HTMLMetaElement | null {
  return document.querySelector<HTMLMetaElement>(selector);
}

export function isDarkTheme(system: boolean): boolean {
  const theme = preference(themeMeta()?.content);
  return theme === "dark" || (theme === "system" && system);
}

function apply(value: ThemePreference) {
  const meta = themeMeta();
  if (meta) meta.content = value === "system" ? "light dark" : value;
}

export function setTheme(value: ThemePreference) {
  apply(value);
  try {
    if (value === "system") localStorage.removeItem(storageKey);
    else localStorage.setItem(storageKey, value);
  } catch {
    // The current page still uses the preference when storage is unavailable.
  }
}

export function observeTheme(onChange: (value: ThemePreference) => void) {
  const frame = requestAnimationFrame(() => onChange(preference(themeMeta()?.content)));
  const sync = (event: StorageEvent) => {
    if (event.key !== storageKey && event.key !== null) return;
    const theme = preference(event.newValue);
    apply(theme);
    onChange(theme);
  };
  addEventListener("storage", sync);
  return () => {
    cancelAnimationFrame(frame);
    removeEventListener("storage", sync);
  };
}
