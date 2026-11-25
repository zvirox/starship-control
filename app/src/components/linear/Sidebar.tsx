import React, { useState } from "react";
import {
  ChevronDown,
  PanelLeftClose,
  PanelLeftOpen,
  Layers,
} from "lucide-react";

export interface SidebarNavItem {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string; strokeWidth?: number }>;
  badge?: string | number;
}

export interface SidebarWorkspaceOption {
  id: string;
  name: string;
  subtitle: string;
}

export interface SidebarProps {
  items: SidebarNavItem[];
  activeId: string;
  onSelect: (id: string) => void;
  workspaces?: SidebarWorkspaceOption[];
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  collapsed?: boolean;
  onToggleCollapse?: () => void;
  footer?: React.ReactNode;
  className?: string;
}

export const Sidebar: React.FC<SidebarProps> = ({
  items,
  activeId,
  onSelect,
  workspaces = [
    { id: "local", name: "Starship Local", subtitle: "Orchestrator node" },
    { id: "hybrid", name: "Hybrid Cloud", subtitle: "External SSE fallback" },
  ],
  activeWorkspaceId = "local",
  onSelectWorkspace,
  collapsed = false,
  onToggleCollapse,
  footer,
  className = "",
}) => {
  const [wsOpen, setWsOpen] = useState(false);
  const currentWs =
    workspaces.find((w) => w.id === activeWorkspaceId) || workspaces[0];

  return (
    <aside
      aria-label="Primary Sidebar"
      className={`shrink-0 flex flex-col justify-between bg-[var(--bg-app)] border-r border-[var(--border)] select-none transition-[width] duration-150 ease-out ${
        collapsed ? "w-[56px]" : "w-[240px]"
      } ${className}`}
    >
      <div className="p-2 space-y-3">
        {/* Workspace Switcher + Collapse Toggle */}
        <div className="relative flex items-center gap-1">
          {!collapsed && (
            <button
              type="button"
              onClick={() => setWsOpen((o) => !o)}
              aria-expanded={wsOpen}
              className="flex-1 min-w-0 h-9 px-2.5 rounded-[6px] bg-[var(--bg-surface)] border border-[var(--border)] hover:border-[var(--border-strong)] flex items-center justify-between gap-2 text-left transition-colors duration-150 focus-ring cursor-pointer"
            >
              <div className="flex items-center gap-2 min-w-0">
                <span className="size-5 rounded-[4px] bg-[var(--tint-blue)] text-[var(--accent-blue)] flex items-center justify-center shrink-0">
                  <Layers className="size-3.5" strokeWidth={1.75} />
                </span>
                <div className="min-w-0">
                  <div className="text-[13px] font-medium text-[var(--text)] truncate leading-tight">
                    {currentWs.name}
                  </div>
                </div>
              </div>
              <ChevronDown
                className="size-3.5 text-[var(--text-muted)] shrink-0"
                strokeWidth={1.75}
              />
            </button>
          )}

          {onToggleCollapse && (
            <button
              type="button"
              onClick={onToggleCollapse}
              aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
              title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
              className="h-9 w-9 shrink-0 inline-flex items-center justify-center rounded-[6px] text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 focus-ring cursor-pointer"
            >
              {collapsed ? (
                <PanelLeftOpen className="size-4" strokeWidth={1.75} />
              ) : (
                <PanelLeftClose className="size-4" strokeWidth={1.75} />
              )}
            </button>
          )}

          {wsOpen && !collapsed && (
            <div className="surface-panel absolute left-0 top-10 w-full rounded-[8px] p-1 z-40">
              {workspaces.map((ws) => (
                <button
                  key={ws.id}
                  type="button"
                  onClick={() => {
                    onSelectWorkspace?.(ws.id);
                    setWsOpen(false);
                  }}
                  className={`w-full text-left px-2.5 py-1.5 rounded-[6px] text-xs transition-colors duration-150 cursor-pointer ${
                    ws.id === currentWs.id
                      ? "bg-[var(--bg-surface-2)] text-[var(--text)] font-medium"
                      : "text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)]"
                  }`}
                >
                  <div className="font-medium truncate">{ws.name}</div>
                  <div className="text-[11px] text-[var(--text-faint)] truncate">
                    {ws.subtitle}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Navigation Items (36px tall) */}
        <nav className="space-y-0.5" aria-label="Workspace sections">
          {items.map((item) => {
            const Icon = item.icon;
            const isActive = item.id === activeId;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onSelect(item.id)}
                aria-current={isActive ? "page" : undefined}
                title={collapsed ? item.label : undefined}
                className={`w-full h-9 flex items-center rounded-[6px] text-[13px] transition-colors duration-150 ease-out focus-ring cursor-pointer ${
                  collapsed ? "justify-center px-0" : "justify-between px-2.5"
                } ${
                  isActive
                    ? "bg-[var(--bg-surface-2)] text-[var(--text)] font-medium border border-[var(--border)]"
                    : "text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] border border-transparent"
                }`}
              >
                <span className="flex items-center gap-2.5 min-w-0">
                  <Icon
                    className={`size-4 shrink-0 ${
                      isActive ? "text-[var(--accent-blue)]" : ""
                    }`}
                    strokeWidth={1.75}
                  />
                  {!collapsed && <span className="truncate">{item.label}</span>}
                </span>
                {!collapsed && item.badge !== undefined && (
                  <span className="px-1.5 py-0.5 text-[11px] font-mono rounded-[4px] bg-[var(--tint-blue)] text-[var(--accent-blue)] tabular-nums">
                    {item.badge}
                  </span>
                )}
              </button>
            );
          })}
        </nav>
      </div>

      {footer && (
        <div className="p-2 border-t border-[var(--border)]">{footer}</div>
      )}
    </aside>
  );
};
