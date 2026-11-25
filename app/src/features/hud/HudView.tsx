import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { GripVertical, X } from "lucide-react";
import { StatusPill, Sparkline } from "../../components/linear";
import "../../stores/themeStore";
import type {
  TrayState,
  HealthStateChangedEvent,
  MissionProgressPayload,
  ApprovalNeededEvent,
} from "../../types/ipc";

export const HudView: React.FC = () => {
  const [statusKind, setStatusKind] = useState<
    "online" | "active" | "warning" | "alert" | "idle"
  >("online");
  const [statusLabel, setStatusLabel] = useState<string>("Ready");
  const [tokensPerSec, setTokensPerSec] = useState<number>(48.4);
  const [tpsHistory, setTpsHistory] = useState<number[]>([
    39, 42, 41, 45, 44, 47, 48.4,
  ]);
  const [isPositive, setIsPositive] = useState<boolean>(true);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenTray: (() => void) | undefined;
    let unlistenHealth: (() => void) | undefined;
    let unlistenApproval: (() => void) | undefined;

    async function bindListeners() {
      unlistenApproval = await listen<ApprovalNeededEvent>(
        "evt://plan/approval_needed",
        () => {
          setStatusKind("alert");
          setStatusLabel("Approval");
        }
      );

      unlistenProgress = await listen<MissionProgressPayload>(
        "evt://mission/progress",
        (event) => {
          if (event.payload.isStalled) {
            setStatusKind("warning");
            setStatusLabel("Stalled");
            setIsPositive(false);
          } else {
            setStatusKind("active");
            setStatusLabel("Executing");
            setIsPositive(true);
          }
        }
      );

      unlistenTray = await listen<{ state: TrayState }>(
        "evt://tray/state",
        (event) => {
          const st = event.payload.state;
          if (st === "idle") {
            setStatusKind("online");
            setStatusLabel("Ready");
            setIsPositive(true);
          } else if (st === "thinking" || st === "executing") {
            setStatusKind("active");
            setStatusLabel(st === "thinking" ? "Thinking" : "Executing");
            setIsPositive(true);
          } else if (st === "stalled" || st === "overloaded") {
            setStatusKind("warning");
            setStatusLabel(st === "stalled" ? "Stalled" : "Overloaded");
            setIsPositive(false);
          } else if (st === "waiting_approval" || st === "failed") {
            setStatusKind("alert");
            setStatusLabel(st === "waiting_approval" ? "Approval" : "Failed");
            setIsPositive(false);
          }
        }
      );

      unlistenHealth = await listen<HealthStateChangedEvent>(
        "evt://health/state_changed",
        (event) => {
          if (event.payload.currentState === "critical") {
            setStatusKind("alert");
            setStatusLabel("Critical");
            setIsPositive(false);
          }
        }
      );
    }

    bindListeners().catch(console.warn);

    const ticker = setInterval(() => {
      setTpsHistory((prev) => {
        const last = prev[prev.length - 1] ?? 45;
        const next = Math.max(
          12,
          Number((last + (Math.random() - 0.48) * 3.2).toFixed(1))
        );
        setTokensPerSec(next);
        return [...prev.slice(1), next];
      });
    }, 2000);

    return () => {
      clearInterval(ticker);
      if (unlistenApproval) unlistenApproval();
      if (unlistenProgress) unlistenProgress();
      if (unlistenTray) unlistenTray();
      if (unlistenHealth) unlistenHealth();
    };
  }, []);

  const handleCloseHud = async () => {
    try {
      await invoke("cmd_toggle_hud");
    } catch {
      // Ignore outside Tauri runtime
    }
  };

  return (
    <div
      role="region"
      aria-label="Floating HUD"
      className="h-screen w-screen bg-[var(--bg-surface)] border border-[var(--border)] rounded-[6px] px-3 flex items-center justify-between gap-2.5 font-sans text-[var(--text)] select-none overflow-hidden"
    >
      {/* Left: Compact drag handle + 20px logo */}
      <div
        data-tauri-drag-region
        className="flex items-center gap-1.5 shrink-0 cursor-grab active:cursor-grabbing"
        title="Drag Floating HUD"
      >
        <GripVertical
          className="size-3.5 text-[var(--text-muted)] pointer-events-none"
          strokeWidth={1.75}
          aria-hidden="true"
        />
        <img
          src="/logo.png"
          alt="Starship Control"
          className="size-5 object-contain pointer-events-none"
        />
      </div>

      {/* Center: StatusPill + tokens/sec + sparkline */}
      <div className="flex items-center gap-3 min-w-0 flex-1 justify-center">
        <StatusPill label={statusLabel} status={statusKind} />

        <div className="flex items-baseline gap-1 shrink-0">
          <span className="text-[14px] font-semibold tabular-nums font-mono text-[var(--text)]">
            {tokensPerSec.toFixed(1)}
          </span>
          <span className="text-[12px] text-[var(--text-muted)]">tok/s</span>
        </div>

        <Sparkline
          data={tpsHistory}
          positive={isPositive}
          width={64}
          height={24}
        />
      </div>

      {/* Right: Close only (no theme toggle — follows Cockpit via Tauri event) */}
      <button
        type="button"
        onClick={handleCloseHud}
        aria-label="Close HUD"
        title="Close HUD"
        className="close-btn size-7 shrink-0 inline-flex items-center justify-center rounded-[6px] text-[var(--text-muted)] transition-colors duration-150 ease-out focus-ring cursor-pointer"
      >
        <X className="size-3.5" strokeWidth={1.75} aria-hidden="true" />
      </button>
    </div>
  );
};
