import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Play,
  RotateCw,
  Terminal,
  XCircle,
  Wifi,
  Sparkles,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type {
  AppTopologyEntry,
  ListAppsResponse,
  TopologyAppStateEvent,
  LaunchAppResponse,
} from "../../types/ipc";

interface AppTopologyViewProps {
  onLaunchSuccess?: (appId: string) => void;
}

export const AppTopologyView: React.FC<AppTopologyViewProps> = ({ onLaunchSuccess }) => {
  const [apps, setApps] = useState<AppTopologyEntry[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [refreshing, setRefreshing] = useState<boolean>(false);
  const [launchingAppId, setLaunchingAppId] = useState<string | null>(null);
  const [pingingAppId, setPingingAppId] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const fetchApps = async () => {
    try {
      setErrorMsg(null);
      const res = await invoke<ListAppsResponse>("cmd_topology_list_apps");
      setApps(res.apps);
    } catch (err: unknown) {
      console.warn("Failed to list topology apps:", err);
      // Fallback ecosystem mock for web preview
      setApps([
        {
          appId: "eep",
          displayName: "Easy Edits Pro",
          version: "0.1.2-alpha.7",
          mode: "live_gui",
          socketPath: "/run/user/1000/starship/bridges/eep.sock",
          binaryPath: "/usr/bin/easy-edits-pro",
          latencyMs: 0.4,
          capabilities: {
            videoEditing: true,
            audioMastering: true,
            timelineControl: true,
            cetTelemetry: true,
          },
        },
        {
          appId: "ssp",
          displayName: "ScreenStudio Pro",
          version: "0.2.0-beta.1",
          mode: "live_gui",
          socketPath: "/run/user/1000/starship/bridges/ssp.sock",
          binaryPath: "/usr/bin/screenstudio-pro",
          latencyMs: 0.6,
          capabilities: {
            screenRecording: true,
            displayCapture: true,
            autoZoom: true,
            cursorTracking: true,
          },
        },
        {
          appId: "alp",
          displayName: "Audio Labs Pro",
          version: "0.1.0-alpha.3",
          mode: "offline",
          socketPath: "/run/user/1000/starship/bridges/alp.sock",
          binaryPath: "/usr/bin/audio-labs-pro",
          latencyMs: null,
          capabilities: {
            voiceStudio: true,
            audioMastering: true,
            noiseRemoval: true,
          },
        },
        {
          appId: "nwp",
          displayName: "Net Watch Pro",
          version: "0.1.1",
          mode: "headless_cli",
          socketPath: "/run/user/1000/starship/bridges/nwp.sock",
          binaryPath: "/usr/bin/net-watch-pro",
          latencyMs: null,
          capabilities: {
            ebpfMonitor: true,
            packetCapture: true,
            latencyTracker: true,
          },
        },
        {
          appId: "atp",
          displayName: "Agentic Tuneup Pro",
          version: "0.1.0",
          mode: "offline",
          socketPath: "/run/user/1000/starship/bridges/atp.sock",
          binaryPath: "/usr/bin/agentic-tuneup-pro",
          latencyMs: null,
          capabilities: {
            packageCleaner: true,
            memoryOptimizer: true,
            cacheCleaner: true,
          },
        },
      ]);
    } finally {
      setLoading(false);
      setRefreshing(false);
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
              prev.map((app) => {
                if (app.appId === update.appId) {
                  return {
                    ...app,
                    mode: update.mode,
                    socketPath: update.socketPath ?? app.socketPath,
                  };
                }
                return app;
              })
            );
          }
        );
      } catch (err) {
        console.warn("Event listen failed:", err);
      }
    }

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleLaunch = async (appId: string) => {
    setLaunchingAppId(appId);
    setErrorMsg(null);
    try {
      await invoke<LaunchAppResponse>("cmd_topology_launch_app", {
        payload: { appId },
      });
      // Optimistically update
      setApps((prev) =>
        prev.map((a) => (a.appId === appId ? { ...a, mode: "live_gui" } : a))
      );
      if (onLaunchSuccess) onLaunchSuccess(appId);
    } catch (err: unknown) {
      console.error(`Failed to launch ${appId}:`, err);
      setErrorMsg(
        typeof err === "object" && err !== null && "userMessage" in err
          ? (err as { userMessage: string }).userMessage
          : `Failed to launch application ${appId}`
      );
    } finally {
      setLaunchingAppId(null);
    }
  };

  const handlePing = async (appId: string) => {
    setPingingAppId(appId);
    setErrorMsg(null);
    try {
      const latency = await invoke<number>("cmd_topology_ping_app", {
        payload: { appId },
      });
      setApps((prev) =>
        prev.map((a) =>
          a.appId === appId ? { ...a, latencyMs: latency, mode: "live_gui" } : a
        )
      );
    } catch (err: unknown) {
      console.warn(`Ping failed for ${appId}:`, err);
    } finally {
      setPingingAppId(null);
    }
  };

  return (
    <div className="flex-1 flex flex-col overflow-y-auto p-6 space-y-6">
      {/* Header Banner */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <Activity className="h-6 w-6 text-cyan-400" />
            <h1 className="text-xl font-bold tracking-tight text-slate-100">
              Ecosystem App Topology Matrix
            </h1>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Zero-configuration discovery via <code className="text-cyan-400 font-mono">$XDG_DATA_HOME/starship/plugins/</code> with kernel <code className="text-cyan-400 font-mono">SO_PEERCRED</code> validation.
          </p>
        </div>

        <button
          onClick={() => {
            setRefreshing(true);
            fetchApps();
          }}
          disabled={refreshing}
          className="flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 hover:bg-white/10 px-3 py-1.5 text-xs text-slate-200 transition-colors disabled:opacity-50"
        >
          <RotateCw className={`h-3.5 w-3.5 text-cyan-400 ${refreshing ? "animate-spin" : ""}`} />
          <span>Scan Plugins</span>
        </button>
      </div>

      {errorMsg && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-300 flex items-center justify-between">
          <span>{errorMsg}</span>
          <button
            onClick={() => setErrorMsg(null)}
            className="text-rose-400 hover:text-rose-200"
          >
            ✕
          </button>
        </div>
      )}

      {/* Grid of Apps */}
      {loading ? (
        <div className="flex items-center justify-center p-12 text-slate-400 text-xs">
          <RotateCw className="h-5 w-5 animate-spin mr-2 text-cyan-400" />
          <span>Scanning XDG manifests and active sockets...</span>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {apps.map((app) => {
            const isLiveGui = app.mode === "live_gui";
            const isHeadless = app.mode === "headless_cli";
            const isOffline = app.mode === "offline";

            return (
              <GlassPanel
                key={app.appId}
                recipe="pane"
                className="p-5 flex flex-col justify-between space-y-4 hover:border-white/20 transition-all group"
              >
                {/* Header */}
                <div className="space-y-2">
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-bold text-slate-100 text-base">
                          {app.displayName}
                        </span>
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded-md bg-white/5 text-slate-400 border border-white/10">
                          v{app.version}
                        </span>
                      </div>
                      <span className="text-[11px] font-mono text-slate-500">
                        appId: {app.appId}
                      </span>
                    </div>

                    {/* Mode Badge */}
                    <div className="flex items-center gap-1.5">
                      {isLiveGui ? (
                        <div className="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-semibold border border-emerald-500/30 bg-emerald-500/15 text-emerald-300 shadow-[0_0_10px_rgba(16,185,129,0.2)]">
                          <span className="relative flex h-2 w-2">
                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                            <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                          </span>
                          <span>Live GUI</span>
                        </div>
                      ) : isHeadless ? (
                        <div className="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-semibold border border-sky-500/30 bg-sky-500/15 text-sky-300">
                          <Terminal className="h-3 w-3 text-sky-400" />
                          <span>Headless CLI</span>
                        </div>
                      ) : (
                        <div className="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium border border-white/10 bg-white/5 text-slate-400">
                          <XCircle className="h-3 w-3 text-slate-500" />
                          <span>Offline</span>
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Latency & Transport Details */}
                  <div className="rounded-xl border border-white/5 bg-black/30 p-2.5 space-y-1.5 text-xs font-mono">
                    <div className="flex items-center justify-between text-slate-400">
                      <span className="text-[11px]">Transport:</span>
                      <span className="text-slate-200">
                        {isLiveGui ? "Unix Socket (SO_PEERCRED)" : "Stdio MCP Pipe"}
                      </span>
                    </div>
                    {app.latencyMs !== null && (
                      <div className="flex items-center justify-between text-slate-400">
                        <span className="text-[11px]">Round-Trip Ping:</span>
                        <span className="text-cyan-400 font-semibold">
                          {app.latencyMs.toFixed(2)} ms
                        </span>
                      </div>
                    )}
                    {app.socketPath && (
                      <div className="text-[10px] text-slate-500 truncate" title={app.socketPath}>
                        Socket: {app.socketPath}
                      </div>
                    )}
                  </div>

                  {/* Capabilities Chips */}
                  <div className="space-y-1.5">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
                      Exposed MCP Capabilities
                    </span>
                    <div className="flex flex-wrap gap-1.5">
                      {Object.keys(app.capabilities).length > 0 ? (
                        Object.keys(app.capabilities).map((cap) => (
                          <span
                            key={cap}
                            className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-mono bg-cyan-500/10 text-cyan-300 border border-cyan-500/20"
                          >
                            <Sparkles className="h-2.5 w-2.5 text-cyan-400" />
                            {cap}
                          </span>
                        ))
                      ) : (
                        <span className="text-[11px] text-slate-500 italic">
                          Standard Tools
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="pt-2 border-t border-white/10 flex items-center justify-between gap-2">
                  <button
                    onClick={() => handlePing(app.appId)}
                    disabled={pingingAppId === app.appId || isOffline}
                    className="flex-1 flex items-center justify-center gap-1.5 rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 px-3 py-1.5 text-xs text-slate-300 transition-colors disabled:opacity-30"
                  >
                    <Wifi className={`h-3 w-3 text-cyan-400 ${pingingAppId === app.appId ? "animate-pulse" : ""}`} />
                    <span>Ping Socket</span>
                  </button>

                  <button
                    onClick={() => handleLaunch(app.appId)}
                    disabled={launchingAppId === app.appId}
                    className={`flex-1 flex items-center justify-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold transition-all ${
                      isLiveGui
                        ? "border border-emerald-500/30 bg-emerald-500/20 text-emerald-200 hover:bg-emerald-500/30"
                        : "bg-cyan-400 hover:bg-cyan-300 text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.25)]"
                    } disabled:opacity-50`}
                  >
                    <Play className="h-3 w-3 fill-current" />
                    <span>
                      {launchingAppId === app.appId
                        ? "Connecting..."
                        : isLiveGui
                        ? "Focus GUI"
                        : "Launch GUI"}
                    </span>
                  </button>
                </div>
              </GlassPanel>
            );
          })}
        </div>
      )}
    </div>
  );
};
