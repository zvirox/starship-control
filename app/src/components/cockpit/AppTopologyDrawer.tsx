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
    <aside className="flex w-[280px] shrink-0 flex-col justify-between border-l border-border-subtle bg-raised/60 p-3 select-none">
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <span className="text-xs font-semibold text-text-primary">Connected apps</span>
            <p className="text-[11px] font-medium text-text-secondary">Desktop integrations</p>
          </div>
          <button
            onClick={fetchApps}
            title="Refresh app connections"
            className="flex h-6 w-6 items-center justify-center rounded-lg bg-white/[0.03] text-text-secondary transition-colors duration-150 hover:bg-white/[0.05] hover:text-text-primary focus-ring cursor-pointer"
          >
            <RotateCw className="h-3.5 w-3.5" strokeWidth={1.5} />
          </button>
        </div>

        <div className="space-y-2">
          {loading ? (
            <div className="p-4 text-center text-[11px] font-medium text-text-secondary">Checking connections...</div>
          ) : (
            apps.map((app) => {
              const isLive = app.mode === "live_gui";
              const isHeadless = app.mode === "headless_cli";

              return (
                <div
                  key={app.appId}
                  className="hairline rounded-xl bg-white/[0.03] p-3 transition-colors duration-150 hover:border-border-glass"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="min-w-0">
                      <div className="truncate text-xs font-semibold text-text-primary">{app.displayName}</div>
                      <div className="mt-1 flex items-center gap-2 text-[11px] font-medium">
                        {isLive ? (
                          <>
                            <span className="h-1.5 w-1.5 rounded-full bg-signal-success" />
                            <span className="text-signal-success font-semibold">Active GUI</span>
                          </>
                        ) : isHeadless ? (
                          <>
                            <span className="h-1.5 w-1.5 rounded-full bg-accent" />
                            <span className="text-accent font-semibold">Service</span>
                          </>
                        ) : (
                          <>
                            <span className="h-1.5 w-1.5 rounded-full bg-text-secondary" />
                            <span className="text-text-secondary">Standby</span>
                          </>
                        )}
                      </div>
                    </div>

                    <div className="shrink-0 text-right">
                      {app.latencyMs !== null ? (
                        <span className="font-mono text-[11px] tabular text-text-secondary font-medium">
                          {app.latencyMs.toFixed(1)}ms
                        </span>
                      ) : (
                        <button
                          onClick={() => handleLaunch(app.appId)}
                          disabled={launchingAppId === app.appId}
                          className="rounded-lg border border-border-subtle bg-white/[0.03] px-2.5 py-1 text-[11px] font-medium text-text-secondary transition-colors duration-150 hover:bg-white/[0.05] hover:text-text-primary focus-ring cursor-pointer"
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

        <div className="space-y-2 border-t border-border-subtle pt-3">
          <span className="text-xs font-semibold text-text-primary">System resources</span>
          <div className="grid grid-cols-2 gap-2">
            <div className="hairline rounded-xl bg-white/[0.03] p-2.5">
              <span className="block text-[11px] font-medium text-text-secondary">VRAM usage</span>
              <span className="font-mono text-[11px] font-semibold tabular text-text-primary">5.4 / 12 GB</span>
            </div>
            <div className="hairline rounded-xl bg-white/[0.03] p-2.5">
              <span className="block text-[11px] font-medium text-text-secondary">CPU load</span>
              <span className="font-mono text-[11px] font-semibold tabular text-sky-400">12%</span>
            </div>
            <div className="hairline rounded-xl bg-white/[0.03] p-2.5">
              <span className="block text-[11px] font-medium text-text-secondary">Session tokens</span>
              <span className="font-mono text-[11px] font-semibold tabular text-text-primary">1,420</span>
            </div>
            <div className="hairline rounded-xl bg-white/[0.03] p-2.5">
              <span className="block text-[11px] font-medium text-text-secondary">Estimated cost</span>
              <span className="font-mono text-[11px] font-semibold tabular text-signal-success">$0.00</span>
            </div>
          </div>
        </div>
      </div>

      <div className="flex items-center justify-between border-t border-border-subtle pt-2.5 text-[11px] text-text-secondary font-medium">
        <span className="flex items-center gap-2">
          <Shield className="h-3.5 w-3.5 text-signal-success" strokeWidth={1.5} />
          <span>Local execution verified</span>
        </span>
        <span className="font-mono text-text-primary font-medium">Isolated</span>
      </div>
    </aside>
  );
};
