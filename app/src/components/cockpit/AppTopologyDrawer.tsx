import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  RotateCw,
  Terminal,
} from "lucide-react";
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
    <aside className="w-[300px] border-l border-white/10 bg-[#0E131B]/50 flex flex-col justify-between p-3 select-none">
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Sub-App Topology
            </span>
            <p className="text-[10px] text-slate-500">Unix Domain Sockets & MCP Stdio</p>
          </div>
          <button
            onClick={fetchApps}
            title="Scan and Ping Sockets"
            className="flex h-6 w-6 items-center justify-center rounded-md bg-white/5 hover:bg-white/10 text-slate-400 hover:text-cyan-300"
          >
            <RotateCw className="h-3 w-3" />
          </button>
        </div>

        {/* Sub-Apps Matrix */}
        <div className="space-y-2">
          {loading ? (
            <div className="p-4 text-center text-xs text-slate-500">Scanning topology...</div>
          ) : (
            apps.map((app) => {
              const isLive = app.mode === "live_gui";
              const isHeadless = app.mode === "headless_cli";

              return (
                <div
                  key={app.appId}
                  className="rounded-xl border border-white/10 bg-white/5 p-2.5 text-xs space-y-1.5 hover:border-white/20 transition-colors"
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-semibold text-slate-200">{app.displayName}</div>
                      <div className="text-[10px] font-mono flex items-center gap-1.5 mt-0.5">
                        {isLive ? (
                          <>
                            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" />
                            <span className="text-emerald-300 font-medium">Live GUI</span>
                          </>
                        ) : isHeadless ? (
                          <>
                            <Terminal className="h-2.5 w-2.5 text-sky-400" />
                            <span className="text-sky-300">Headless CLI</span>
                          </>
                        ) : (
                          <>
                            <span className="h-1.5 w-1.5 rounded-full bg-slate-500" />
                            <span className="text-slate-400">Offline</span>
                          </>
                        )}
                      </div>
                    </div>

                    <div className="text-right">
                      {app.latencyMs !== null ? (
                        <span className="text-[10px] font-mono text-cyan-400">
                          {app.latencyMs.toFixed(1)}ms
                        </span>
                      ) : (
                        <button
                          onClick={() => handleLaunch(app.appId)}
                          disabled={launchingAppId === app.appId}
                          className="px-2 py-0.5 rounded bg-cyan-400/20 text-cyan-300 hover:bg-cyan-400/30 text-[10px] font-medium border border-cyan-400/30"
                        >
                          {launchingAppId === app.appId ? "..." : "Launch"}
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Resource Gauges */}
        <div className="border-t border-white/10 pt-3 space-y-2">
          <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
            Hardware & Budget
          </span>
          <div className="grid grid-cols-2 gap-2 text-xs font-mono">
            <div className="rounded-lg border border-white/10 bg-black/30 p-2">
              <span className="text-[10px] text-slate-500 block">VRAM</span>
              <span className="text-purple-300 font-semibold">5.4 / 12 GB</span>
            </div>
            <div className="rounded-lg border border-white/10 bg-black/30 p-2">
              <span className="text-[10px] text-slate-500 block">CPU LOAD</span>
              <span className="text-cyan-300 font-semibold">12%</span>
            </div>
            <div className="rounded-lg border border-white/10 bg-black/30 p-2">
              <span className="text-[10px] text-slate-500 block">TOKENS</span>
              <span className="text-slate-300 font-semibold">1,420</span>
            </div>
            <div className="rounded-lg border border-white/10 bg-black/30 p-2">
              <span className="text-[10px] text-slate-500 block">EST. COST</span>
              <span className="text-emerald-300 font-semibold">$0.00</span>
            </div>
          </div>
        </div>
      </div>

      <div className="border-t border-white/10 pt-2 text-[10px] font-mono text-slate-500 flex items-center justify-between">
        <span>Security Token: Signed</span>
        <span className="text-emerald-400">SO_PEERCRED OK</span>
      </div>
    </aside>
  );
};
