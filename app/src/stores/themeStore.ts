import { create } from "zustand";
import { emit, listen } from "@tauri-apps/api/event";

export type ThemeMode = "dark" | "light";

interface ThemeState {
  theme: ThemeMode;
  setTheme: (theme: ThemeMode, broadcast?: boolean) => void;
  toggleTheme: () => void;
}

const THEME_STORAGE_KEY = "ui-theme";
const THEME_EVENT = "evt://theme/changed";

function applyThemeToDocument(theme: ThemeMode) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.setAttribute("data-theme", theme);
  root.style.colorScheme = theme;
}

function getInitialTheme(): ThemeMode {
  if (typeof window !== "undefined") {
    try {
      const saved = localStorage.getItem(THEME_STORAGE_KEY);
      if (saved === "light" || saved === "dark") {
        return saved;
      }
    } catch {
      // Ignore storage access errors
    }
    if (
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
    ) {
      return "dark";
    }
    return "light";
  }
  return "dark";
}

const initialTheme = getInitialTheme();
applyThemeToDocument(initialTheme);

const broadcastChannel =
  typeof window !== "undefined" && typeof BroadcastChannel !== "undefined"
    ? new BroadcastChannel("starship_theme_sync")
    : null;

function broadcastThemeChange(theme: ThemeMode) {
  if (typeof window !== "undefined") {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Storage access may fail in restricted contexts
    }
  }

  if (broadcastChannel) {
    try {
      broadcastChannel.postMessage({ theme });
    } catch {
      // Ignore broadcast errors
    }
  }

  emit(THEME_EVENT, { theme }).catch(() => {
    // Running outside Tauri or listener unavailable
  });
}

export const useThemeStore = create<ThemeState>((set) => ({
  theme: initialTheme,
  setTheme: (newTheme: ThemeMode, broadcast = true) => {
    applyThemeToDocument(newTheme);
    if (broadcast) {
      broadcastThemeChange(newTheme);
    }
    set({ theme: newTheme });
  },
  toggleTheme: () => {
    set((state) => {
      const nextTheme: ThemeMode = state.theme === "dark" ? "light" : "dark";
      applyThemeToDocument(nextTheme);
      broadcastThemeChange(nextTheme);
      return { theme: nextTheme };
    });
  },
}));

if (typeof window !== "undefined") {
  if (broadcastChannel) {
    broadcastChannel.onmessage = (event) => {
      const newTheme = event.data?.theme;
      if (newTheme === "light" || newTheme === "dark") {
        applyThemeToDocument(newTheme);
        useThemeStore.setState({ theme: newTheme });
      }
    };
  }

  window.addEventListener("storage", (e) => {
    if (e.key === THEME_STORAGE_KEY) {
      const newTheme = e.newValue as ThemeMode | null;
      if (newTheme === "light" || newTheme === "dark") {
        applyThemeToDocument(newTheme);
        useThemeStore.setState({ theme: newTheme });
      }
    }
  });

  listen<{ theme: ThemeMode }>(THEME_EVENT, (event) => {
    const newTheme = event.payload?.theme;
    if (newTheme === "light" || newTheme === "dark") {
      applyThemeToDocument(newTheme);
      useThemeStore.setState({ theme: newTheme });
    }
  }).catch(() => {
    // Ignore outside Tauri runtime
  });
}
