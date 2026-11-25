import React from "react";
import { Sun, Moon } from "lucide-react";
import { useThemeStore } from "../../stores/themeStore";

export interface ThemeToggleProps {
  className?: string;
}

export const ThemeToggle: React.FC<ThemeToggleProps> = ({ className = "" }) => {
  const { theme, toggleTheme } = useThemeStore();

  return (
    <button
      type="button"
      onClick={toggleTheme}
      aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
      title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
      className={`size-7 inline-flex items-center justify-center rounded-[6px] text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 ease-out focus-ring cursor-pointer ${className}`}
    >
      {theme === "dark" ? (
        <Sun className="size-3.5" strokeWidth={1.75} aria-hidden="true" />
      ) : (
        <Moon className="size-3.5" strokeWidth={1.75} aria-hidden="true" />
      )}
    </button>
  );
};
