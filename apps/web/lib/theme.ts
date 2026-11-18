"use client";

import { useCallback, useSyncExternalStore } from "react";

export type ThemeMode = "light" | "dark" | "system";

const KEY = "orbit.theme";
const listeners = new Set<() => void>();

function read(): ThemeMode {
  try {
    const stored = window.localStorage.getItem(KEY);
    return stored === "dark" || stored === "light" ? stored : "system";
  } catch {
    return "system";
  }
}

function apply(mode: ThemeMode) {
  const dark =
    mode === "dark" ||
    (mode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", dark);
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const onSystemChange = () => {
    if (read() === "system") apply("system");
  };
  // Another tab changed the preference.
  const onStorage = (event: StorageEvent) => {
    if (event.key === KEY) {
      apply(read());
      listener();
    }
  };
  media.addEventListener("change", onSystemChange);
  window.addEventListener("storage", onStorage);
  return () => {
    listeners.delete(listener);
    media.removeEventListener("change", onSystemChange);
    window.removeEventListener("storage", onStorage);
  };
}

/**
 * Theme preference, shared by every control that shows or changes it (account menu,
 * command palette). "system" follows the OS and is stored as the absence of a key.
 */
export function useTheme() {
  const mode = useSyncExternalStore<ThemeMode>(subscribe, read, () => "system");

  const setMode = useCallback((next: ThemeMode) => {
    try {
      if (next === "system") window.localStorage.removeItem(KEY);
      else window.localStorage.setItem(KEY, next);
    } catch {
      // Storage may be unavailable; the theme still applies for this page view.
    }
    apply(next);
    listeners.forEach((listener) => listener());
  }, []);

  return { mode, setMode };
}
