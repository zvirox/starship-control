import { create } from "zustand";
import { emit, listen } from "@tauri-apps/api/event";

export type ThemeMode = "dark" | "light";

interface ThemeState {
  theme: ThemeMode;
  setTheme: (theme: ThemeMode, broadcast?: boolean) => void;
  toggleTheme: () => void;
}

const THEME_STORAGE_KEY = "starship_theme";
const THEME_EVENT = "evt://theme/changed";

// Helper to apply theme classes and color-scheme to <html>
function applyThemeToDocument(theme: ThemeMode) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (theme === "light") {
    root.classList.remove("dark");
    root.classList.add("light");
    root.style.colorScheme = "light";
  } else {
    root.classList.remove("light");
    root.classList.add("dark");
    root.style.colorScheme = "dark";
  }
}

// Initial theme detection
function getInitialTheme(): ThemeMode {
  if (typeof window !== "undefined") {
    const saved = localStorage.getItem(THEME_STORAGE_KEY) as ThemeMode | null;
    if (saved === "light" || saved === "dark") {
      return saved;
    }
  }
  return "dark"; // Default to graphite dark
}

const initialTheme = getInitialTheme();
applyThemeToDocument(initialTheme);

// Multi-window communication: BroadcastChannel
const broadcastChannel =
  typeof window !== "undefined" && typeof BroadcastChannel !== "undefined"
    ? new BroadcastChannel("starship_theme_sync")
    : null;

function broadcastThemeChange(theme: ThemeMode) {
  if (typeof window !== "undefined") {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Storage access may fail in private contexts
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
    // Tauri event bus fallback
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
      const nextTheme = state.theme === "dark" ? "light" : "dark";
      applyThemeToDocument(nextTheme);
      broadcastThemeChange(nextTheme);
      return { theme: nextTheme };
    });
  },
}));

// Cross-window reactivity handlers
if (typeof window !== "undefined") {
  // 1. BroadcastChannel sync (immediate across webview contexts)
  if (broadcastChannel) {
    broadcastChannel.onmessage = (event) => {
      const newTheme = event.data?.theme;
      if (newTheme === "light" || newTheme === "dark") {
        applyThemeToDocument(newTheme);
        useThemeStore.setState({ theme: newTheme });
      }
    };
  }

  // 2. Storage event listener (standard multi-window storage sync)
  window.addEventListener("storage", (e) => {
    if (e.key === THEME_STORAGE_KEY) {
      const newTheme = e.newValue as ThemeMode | null;
      if (newTheme === "light" || newTheme === "dark") {
        applyThemeToDocument(newTheme);
        useThemeStore.setState({ theme: newTheme });
      }
    }
  });

  // 3. Tauri event bus listener (system-wide window sync)
  listen<{ theme: ThemeMode }>(THEME_EVENT, (event) => {
    const newTheme = event.payload?.theme;
    if (newTheme === "light" || newTheme === "dark") {
      applyThemeToDocument(newTheme);
      useThemeStore.setState({ theme: newTheme });
    }
  }).catch(() => {
    // Ignore if running outside Tauri context
  });
}
