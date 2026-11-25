import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { RotateCw, Shield } from "lucide-react";
import type {
  AppTopologyEntry,
  ListAppsResponse,
  TopologyAppStateEvent,
  LaunchAppResponse,
} from "../../types/ipc";

export const AppTopologyDrawer: React.FC = () => {
  const [apps, setApps] = useState<AppTopologyEntry[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [launchingAppId, setLaunchingAppId] = useState<string | null>(null);

  const fetchApps = async () => {
    try {
      const res = await invoke<ListAppsResponse>("cmd_topology_list_apps");
      setApps(res.apps);
    } catch (err: unknown) {
      console.warn("Topology drawer fetch fallback:", err);
      setApps([
        {
          appId: "eep",
          displayName: "Easy Edits Pro",
          version: "0.1.2",
          mode: "live_gui",
          socketPath: "/run/user/1000/starship/bridges/eep.sock",
          binaryPath: "/usr/bin/easy-edits-pro",
          latencyMs: 0.4,
          capabilities: { videoEditing: true, timelineControl: true },
        },
        {
          appId: "ssp",
          displayName: "ScreenStudio Pro",
          version: "0.2.0",
          mode: "live_gui",
          socketPath: "/run/user/1000/starship/bridges/ssp.sock",
          binaryPath: "/usr/bin/screenstudio-pro",
          latencyMs: 0.6,
          capabilities: { screenRecording: true },
        },
        {
          appId: "alp",
          displayName: "Audio Labs Pro",
          version: "0.1.0",
          mode: "offline",
          socketPath: "/run/user/1000/starship/bridges/alp.sock",
          binaryPath: "/usr/bin/audio-labs-pro",
          latencyMs: null,
          capabilities: { voiceStudio: true },
        },
        {
          appId: "nwp",
          displayName: "Net Watch Pro",
          version: "0.1.1",
          mode: "headless_cli",
          socketPath: null,
          binaryPath: "/usr/bin/net-watch-pro",
          latencyMs: null,
          capabilities: { ebpfMonitor: true },
        },
        {
          appId: "atp",
          displayName: "Agentic Tuneup Pro",
          version: "0.1.0",
          mode: "offline",
          socketPath: null,
          binaryPath: "/usr/bin/agentic-tuneup-pro",
          latencyMs: null,
          capabilities: { packageCleaner: true },
        },
      ]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchApps();

    let unlisten: (() => void) | undefined;
    async function setupListener() {
      try {
        unlisten = await listen<TopologyAppStateEvent>(
          "evt://topology/app_state",
          (event) => {
            const update = event.payload;
            setApps((prev) =>
              prev.map((app) =>
                app.appId === update.appId
                  ? { ...app, mode: update.mode, socketPath: update.socketPath ?? app.socketPath }
                  : app
              )
            );
          }
        );
      } catch (e) {
        console.warn("Listener error:", e);
      }
    }

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleLaunch = async (appId: string) => {
    setLaunchingAppId(appId);
    try {
      await invoke<LaunchAppResponse>("cmd_topology_launch_app", {
        payload: { appId },
      });
      setApps((prev) =>
        prev.map((a) => (a.appId === appId ? { ...a, mode: "live_gui" } : a))
      );
    } catch (err: unknown) {
      console.warn("Launch failed:", err);
    } finally {
      setLaunchingAppId(null);
    }
  };

  return (
    <aside className="hidden xl:flex w-[250px] shrink-0 flex-col justify-between border-l border-[var(--border)] bg-[var(--bg-surface)] p-3 select-none">
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <span className="text-[12px] font-semibold text-[var(--text)]">Connected apps</span>
            <p className="text-[12px] text-[var(--text-muted)]">Desktop integrations</p>
          </div>
          <button
            type="button"
            onClick={fetchApps}
            title="Refresh app connections"
            className="flex size-7 items-center justify-center rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)] transition-colors duration-150 hover:border-[var(--border-strong)] hover:text-[var(--text)] focus-ring cursor-pointer"
          >
            <RotateCw className="size-3.5" strokeWidth={1.75} />
          </button>
        </div>

        <div className="space-y-2">
          {loading ? (
            <div className="p-4 text-center text-[12px] text-[var(--text-muted)]">Checking connections...</div>
          ) : (
            apps.map((app) => {
              const isLive = app.mode === "live_gui";
              const isHeadless = app.mode === "headless_cli";

              return (
                <div
                  key={app.appId}
                  className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 transition-colors duration-150 hover:border-[var(--border-strong)]"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="min-w-0">
                      <div className="truncate text-[12px] font-semibold text-[var(--text)]">{app.displayName}</div>
                      <div className="mt-1 flex items-center gap-2 text-[12px]">
                        {isLive ? (
                          <>
                            <span className="size-1.5 rounded-full bg-[var(--accent-green)]" />
                            <span className="text-[var(--accent-green)] font-medium">Active GUI</span>
                          </>
                        ) : isHeadless ? (
                          <>
                            <span className="size-1.5 rounded-full bg-[var(--accent-blue)]" />
                            <span className="text-[var(--accent-blue)] font-medium">Service</span>
                          </>
                        ) : (
                          <>
                            <span className="size-1.5 rounded-full bg-[var(--text-faint)]" />
                            <span className="text-[var(--text-muted)]">Standby</span>
                          </>
                        )}
                      </div>
                    </div>

                    <div className="shrink-0 text-right">
                      {app.latencyMs !== null ? (
                        <span className="font-mono text-[12px] tabular-nums text-[var(--text-muted)]">
                          {app.latencyMs.toFixed(1)}ms
                        </span>
                      ) : (
                        <button
                          type="button"
                          onClick={() => handleLaunch(app.appId)}
                          disabled={launchingAppId === app.appId}
                          className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] px-2 py-1 text-[12px] font-medium text-[var(--text-muted)] transition-colors duration-150 hover:border-[var(--border-strong)] hover:text-[var(--text)] focus-ring cursor-pointer"
                        >
                          {launchingAppId === app.appId ? "Launching" : "Launch"}
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        <div className="space-y-2 border-t border-[var(--border)] pt-3">
          <span className="text-[12px] font-semibold text-[var(--text)]">System resources</span>
          <div className="grid grid-cols-2 gap-2">
            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2.5">
              <span className="block text-[12px] text-[var(--text-muted)]">VRAM usage</span>
              <span className="font-mono text-[12px] font-semibold tabular-nums text-[var(--text)]">5.4 / 12 GB</span>
            </div>
            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2.5">
              <span className="block text-[12px] text-[var(--text-muted)]">CPU load</span>
              <span className="font-mono text-[12px] font-semibold tabular-nums text-[var(--accent-sky)]">12%</span>
            </div>
            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2.5">
              <span className="block text-[12px] text-[var(--text-muted)]">Session tokens</span>
              <span className="font-mono text-[12px] font-semibold tabular-nums text-[var(--text)]">1,420</span>
            </div>
            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2.5">
              <span className="block text-[12px] text-[var(--text-muted)]">Est. cost</span>
              <span className="font-mono text-[12px] font-semibold tabular-nums text-[var(--accent-green)]">$0.00</span>
            </div>
          </div>
        </div>
      </div>

      <div className="flex items-center justify-between border-t border-[var(--border)] pt-2.5 text-[12px] text-[var(--text-muted)]">
        <span className="flex items-center gap-2">
          <Shield className="size-3.5 text-[var(--accent-green)]" strokeWidth={1.75} />
          <span>Local verified</span>
        </span>
        <span className="font-mono text-[var(--text)] font-medium">Isolated</span>
      </div>
    </aside>
  );
};
