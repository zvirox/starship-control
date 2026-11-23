import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Activity, Play, RotateCw, Wifi, X } from "lucide-react";
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
    <div className="flex-1 flex flex-col overflow-y-auto p-6 gap-6 font-sans">
      {/* Header Banner */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Activity className="h-4 w-4 text-accent" strokeWidth={1.5} />
            <h1 className="text-lg font-medium text-text-primary">
              Ecosystem app topology matrix
            </h1>
          </div>
          <p className="text-xs text-text-secondary">
            Zero-configuration discovery via{" "}
            <code className="font-mono text-text-primary">
              $XDG_DATA_HOME/starship/plugins/
            </code>{" "}
            with kernel <code className="font-mono text-text-primary">SO_PEERCRED</code> validation
          </p>
        </div>

        <button
          onClick={() => {
            setRefreshing(true);
            fetchApps();
          }}
          disabled={refreshing}
          className="flex items-center gap-2 rounded-lg border border-border-subtle bg-white/[0.03] hover:bg-white/10 px-3 py-2 text-xs text-text-secondary transition-colors disabled:opacity-40 focus-visible:outline-none focus-visible:text-accent"
        >
          <RotateCw
            className={`h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`}
            strokeWidth={1.5}
          />
          <span>Scan plugins</span>
        </button>
      </div>

      {errorMsg && (
        <div className="flex items-center justify-between rounded-xl border border-signal-gate/30 bg-signal-gate-bg p-3 text-xs text-text-primary">
          <span>{errorMsg}</span>
          <button
            onClick={() => setErrorMsg(null)}
            aria-label="Dismiss error"
            className="text-signal-gate hover:text-text-primary transition-colors focus-visible:outline-none focus-visible:text-accent"
          >
            <X className="h-4 w-4" strokeWidth={1.5} />
          </button>
        </div>
      )}

      {/* Grid of Apps */}
      {loading ? (
        <div className="flex items-center justify-center p-12 text-text-secondary text-xs">
          <RotateCw className="h-4 w-4 animate-spin mr-2 text-accent" strokeWidth={1.5} />
          <span>Scanning XDG manifests and active sockets</span>
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
                className="p-5 flex flex-col justify-between gap-4 hover:border-border-glass transition-colors"
              >
                {/* Header */}
                <div className="space-y-3">
                  <div className="flex items-start justify-between">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-text-primary">
                          {app.displayName}
                        </span>
                        <span className="quiet-chip font-mono">v{app.version}</span>
                      </div>
                      <span className="text-[11px] font-mono text-text-disabled">
                        appId: {app.appId}
                      </span>
                    </div>

                    {/* Mode Badge */}
                    <div className="flex items-center gap-1.5">
                      {isLiveGui ? (
                        <span className="quiet-chip">
                          <span className="h-1.5 w-1.5 rounded-full bg-signal-success" />
                          Live GUI
                        </span>
                      ) : isHeadless ? (
                        <span className="quiet-chip">
                          <span className="h-1.5 w-1.5 rounded-full bg-accent" />
                          Headless CLI
                        </span>
                      ) : (
                        <span className="quiet-chip">
                          <span className="h-1.5 w-1.5 rounded-full bg-signal-gate" />
                          Offline
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Latency & Transport Details */}
                  <div className="rounded-xl border border-border-subtle bg-black/30 p-3 gap-2 font-mono text-xs">
                    <div className="flex items-center justify-between text-text-secondary">
                      <span className="text-[11px] text-text-disabled">Transport:</span>
                      <span className="text-text-primary">
                        {isLiveGui ? "Unix socket (SO_PEERCRED)" : "Stdio MCP pipe"}
                      </span>
                    </div>
                    {app.latencyMs !== null && (
                      <div className="flex items-center justify-between text-text-secondary">
                        <span className="text-[11px] text-text-disabled">Round-trip ping:</span>
                        <span className="text-accent tabular">{app.latencyMs.toFixed(2)} ms</span>
                      </div>
                    )}
                    {app.socketPath && (
                      <div
                        className="text-[11px] text-text-disabled truncate"
                        title={app.socketPath}
                      >
                        Socket: {app.socketPath}
                      </div>
                    )}
                  </div>

                  {/* Capabilities Chips */}
                  <div className="space-y-2">
                    <span className="block text-[11px] font-medium text-text-secondary">
                      Exposed MCP capabilities
                    </span>
                    <div className="flex flex-wrap gap-2">
                      {Object.keys(app.capabilities).length > 0 ? (
                        Object.keys(app.capabilities).map((cap) => (
                          <span
                            key={cap}
                            className="inline-flex items-center rounded-lg border border-border-subtle bg-white/[0.03] px-2 py-1 font-mono text-[11px] text-text-secondary"
                          >
                            {cap}
                          </span>
                        ))
                      ) : (
                        <span className="text-[11px] text-text-disabled">Standard tools</span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="pt-3 border-t border-border-subtle flex items-center gap-2">
                  <button
                    onClick={() => handlePing(app.appId)}
                    disabled={pingingAppId === app.appId || isOffline}
                    className="flex-1 flex items-center justify-center gap-1.5 rounded-lg border border-border-subtle bg-white/[0.03] hover:bg-white/10 px-3 py-2 text-xs text-text-secondary transition-colors disabled:opacity-40 focus-visible:outline-none focus-visible:text-accent"
                  >
                    <Wifi className="h-3.5 w-3.5" strokeWidth={1.5} />
                    <span>Ping socket</span>
                  </button>

                  <button
                    onClick={() => handleLaunch(app.appId)}
                    disabled={launchingAppId === app.appId}
                    className={`flex-1 flex items-center justify-center gap-1.5 rounded-lg px-3 py-2 text-xs font-medium transition-colors focus-visible:outline-none disabled:opacity-40 ${
                      isLiveGui
                        ? "border border-border-subtle bg-white/[0.03] hover:bg-white/10 text-text-primary"
                        : "bg-accent hover:bg-accent-hover text-accent-text"
                    }`}
                  >
                    <Play className="h-3.5 w-3.5" strokeWidth={1.5} />
                    <span>
                      {launchingAppId === app.appId
                        ? "Connecting"
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
