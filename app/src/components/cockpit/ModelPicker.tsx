import React, { useMemo, useRef, useState } from "react";
import { Check, ChevronDown, Cpu, Search, Star, RefreshCw, Plus } from "lucide-react";
import type { ModelRecord } from "../../types/ipc";

export interface ModelPickerProps {
  models: ModelRecord[];
  activeModel: ModelRecord | null;
  onSelect: (model: ModelRecord) => void;
  onRefresh: () => void;
  onOpenSettings: () => void;
  onToggleFavorite: (model: ModelRecord) => void;
}

const GROUP_LABELS: Record<string, string> = {
  local_gguf: "Local",
  cloud_openrouter: "OpenRouter",
  cloud_openai: "OpenAI",
  cloud_anthropic: "Anthropic",
  cloud_gemini: "Gemini",
};

const PAGE_SIZE = 50;

/// Searchable, grouped model selector. Large catalogs are windowed: only the
/// first N matching rows render until the list is scrolled near the bottom.
export const ModelPicker: React.FC<ModelPickerProps> = ({
  models,
  activeModel,
  onSelect,
  onRefresh,
  onOpenSettings,
  onToggleFavorite,
}) => {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [visible, setVisible] = useState(PAGE_SIZE);
  const listRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const ordered = [...models].sort((a, b) =>
      Number(b.is_favorite) - Number(a.is_favorite),
    );
    return q ? ordered.filter((m) => m.id.toLowerCase().includes(q) || m.name.toLowerCase().includes(q)) : ordered;
  }, [models, query]);

  const shown = filtered.slice(0, visible);
  const favorites = shown.filter((m) => m.is_favorite);
  const rest = shown.filter((m) => !m.is_favorite);

  const groups = useMemo(() => {
    const byProvider = new Map<string, ModelRecord[]>();
    for (const model of rest) {
      const key = model.provider_type;
      byProvider.set(key, [...(byProvider.get(key) ?? []), model]);
    }
    return [...byProvider.entries()];
  }, [rest]);

  const handleScroll = () => {
    const element = listRef.current;
    if (!element) return;
    if (element.scrollHeight - element.scrollTop - element.clientHeight < 80) {
      setVisible((count) => count + PAGE_SIZE);
    }
  };

  const choose = (model: ModelRecord) => {
    onSelect(model);
    setOpen(false);
    setQuery("");
  };

  const row = (model: ModelRecord) => {
    const notDownloaded =
      model.provider_type === "local_gguf" && model.download_status !== "ready";
    return (
    <div
      key={model.id}
      className={`flex items-center gap-2 rounded-[6px] px-2 py-1.5 text-[12px] transition-colors duration-150 ${
        activeModel?.id === model.id
          ? "bg-[var(--tint-blue)] text-[var(--accent-blue)]"
          : "text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)]"
      }`}
    >
      <button
        type="button"
        disabled={notDownloaded}
        title={notDownloaded ? "Not downloaded — use the Models tab to download it" : undefined}
        className="flex min-w-0 flex-1 items-center justify-between gap-2 text-left focus-ring cursor-pointer disabled:opacity-50"
        onClick={() => choose(model)}
      >
        <span className="flex min-w-0 items-center gap-1.5">
          {activeModel?.id === model.id && <Check className="size-3 shrink-0" strokeWidth={2} />}
          <span className="truncate font-medium">{model.name}</span>
          {notDownloaded && <span className="text-[10px] text-[var(--text-faint)]">Not downloaded</span>}
        </span>
        <span className="flex shrink-0 items-center gap-1.5 font-mono text-[10px] text-[var(--text-faint)]">
          {model.supports_tools && <span title="Supports tools">tools ✓</span>}
          {model.context_window > 0 && <span>{(model.context_window / 1000).toFixed(0)}k</span>}
          {model.input_price > 0 && <span>${(model.input_price * 1_000_000).toFixed(2)}/M</span>}
        </span>
      </button>
      <button
        type="button"
        className="shrink-0 rounded p-0.5 focus-ring cursor-pointer"
        title={model.is_favorite ? "Unpin model" : "Pin model"}
        onClick={() => onToggleFavorite(model)}
      >
        <Star
          className={`size-3 ${model.is_favorite ? "fill-[var(--accent-amber)] text-[var(--accent-amber)]" : ""}`}
          strokeWidth={1.75}
        />
      </button>
    </div>
    );
  };

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        className="h-7 flex items-center gap-2 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2.5 text-[12px] text-[var(--text)] hover:border-[var(--border-strong)] transition-colors duration-150 focus-ring cursor-pointer"
        aria-label="Choose a model"
      >
        <Cpu className="size-3.5 text-[var(--accent-blue)]" strokeWidth={1.75} />
        <span className="font-medium truncate max-w-44">
          {activeModel?.name || "Select model"}
        </span>
        <ChevronDown className="size-3.5 text-[var(--text-muted)]" strokeWidth={1.75} />
      </button>

      {open && (
        <div className="surface-panel absolute right-0 z-50 mt-1 w-80 rounded-[8px] border border-[var(--border-strong)] p-2">
          <div className="mb-2 flex items-center gap-2">
            <div className="relative flex-1">
              <Search
                className="absolute left-2 top-2 size-3.5 text-[var(--text-muted)]"
                strokeWidth={1.75}
              />
              <input
                autoFocus
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value);
                  setVisible(PAGE_SIZE);
                }}
                placeholder="Search models…"
                className="h-7 w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] pl-7 pr-2 text-[12px] text-[var(--text)] focus-ring"
              />
            </div>
            <button
              type="button"
              onClick={onRefresh}
              className="rounded-[6px] border border-[var(--border)] p-1.5 text-[var(--text-muted)] hover:text-[var(--text)] focus-ring cursor-pointer"
              title="Refresh provider catalogs"
            >
              <RefreshCw className="size-3.5" strokeWidth={1.75} />
            </button>
          </div>

          <div
            ref={listRef}
            onScroll={handleScroll}
            className="max-h-72 overflow-y-auto"
          >
            {favorites.length > 0 && (
              <div className="mb-1">
                <div className="px-2 py-1 text-[11px] font-medium uppercase tracking-wider text-[var(--text-faint)]">
                  Favorites
                </div>
                {favorites.map(row)}
              </div>
            )}
            {groups.map(([provider, entries]) => (
              <div key={provider} className="mb-1">
                <div className="px-2 py-1 text-[11px] font-medium uppercase tracking-wider text-[var(--text-faint)]">
                  {GROUP_LABELS[provider] ?? provider}
                </div>
                {entries.map(row)}
              </div>
            ))}
            {shown.length < filtered.length && (
              <div className="px-2 py-1 text-center text-[11px] text-[var(--text-faint)]">
                Showing {shown.length} of {filtered.length} — scroll for more
              </div>
            )}
            {filtered.length === 0 && (
              <div className="px-2 py-3 text-center text-[12px] text-[var(--text-muted)]">
                No models match.
              </div>
            )}
          </div>

          <button
            type="button"
            onClick={() => {
              setOpen(false);
              onOpenSettings();
            }}
            className="mt-2 flex w-full items-center gap-2 rounded-[6px] border border-[var(--border)] px-2 py-1.5 text-left text-[12px] text-[var(--text-muted)] hover:text-[var(--text)] focus-ring cursor-pointer"
          >
            <Plus className="size-3.5" strokeWidth={1.75} />
            Not configured → Add API key
          </button>
        </div>
      )}
    </div>
  );
};
