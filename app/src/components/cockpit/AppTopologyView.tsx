import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Activity, Play, RotateCw, Wifi, X } from "lucide-react";
import { LinearCard, LinearButton, StatusPill } from "../linear";
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
      const core = err as { userMessage?: unknown } | null;
      const message =
        core && typeof core.userMessage === "string"
          ? core.userMessage
          : err instanceof Error
            ? err.message
            : String(err);
      setErrorMsg(message);
      setApps([]);
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
      const response = await invoke<LaunchAppResponse>("cmd_topology_launch_app", {
        payload: { appId },
      });
      const mode =
        response.bridgeStatus === "launched_no_bridge" ? "headless_cli" : "live_gui";
      setApps((prev) =>
        prev.map((a) => (a.appId === appId ? { ...a, mode } : a))
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
    <div className="flex-1 flex flex-col overflow-y-auto bg-[var(--bg-app)] p-5 gap-5 font-sans">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Activity className="size-4 text-[var(--accent-blue)]" strokeWidth={1.75} />
            <h1 className="text-[16px] font-semibold text-[var(--text)]">
              Ecosystem app topology matrix
            </h1>
          </div>
          <p className="text-[12px] text-[var(--text-muted)]">
            Zero-configuration discovery via{" "}
            <code className="font-mono text-[var(--text)]">$XDG_DATA_HOME/starship/plugins/</code>{" "}
            with kernel <code className="font-mono text-[var(--text)]">SO_PEERCRED</code> validation
          </p>
        </div>

        <LinearButton
          variant="secondary"
          size="sm"
          onClick={() => {
            setRefreshing(true);
            fetchApps();
          }}
          disabled={refreshing}
        >
          <RotateCw className={`size-3.5 ${refreshing ? "animate-spin" : ""}`} strokeWidth={1.75} />
          <span>Scan plugins</span>
        </LinearButton>
      </div>

      {errorMsg && (
        <div className="flex items-center justify-between rounded-[8px] border border-[var(--accent-red)] bg-[var(--tint-red)] p-3 text-[12px] text-[var(--text)]">
          <span>{errorMsg}</span>
          <button
            type="button"
            onClick={() => setErrorMsg(null)}
            aria-label="Dismiss error"
            className="text-[var(--accent-red)] hover:text-[var(--text)] transition-colors focus-ring cursor-pointer"
          >
            <X className="size-4" strokeWidth={1.75} />
          </button>
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center p-12 text-[var(--text-muted)] text-[12px]">
          <RotateCw className="size-4 animate-spin mr-2 text-[var(--accent-blue)]" strokeWidth={1.75} />
          <span>Scanning XDG manifests and active sockets</span>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {apps.map((app) => {
            const isLiveGui = app.mode === "live_gui";
            const isHeadless = app.mode === "headless_cli";
            const isOffline = app.mode === "offline";

            return (
              <LinearCard
                key={app.appId}
                level={1}
                className="p-5 flex flex-col justify-between gap-4"
              >
                <div className="space-y-3">
                  <div className="flex items-start justify-between gap-2">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="text-[14px] font-semibold text-[var(--text)]">
                          {app.displayName}
                        </span>
                        <span className="quiet-chip font-mono">v{app.version}</span>
                      </div>
                      <span className="text-[12px] font-mono text-[var(--text-muted)]">
                        appId: {app.appId}
                      </span>
                    </div>

                    <StatusPill
                      label={isLiveGui ? "Live GUI" : isHeadless ? "Headless" : "Offline"}
                      status={isLiveGui ? "online" : isHeadless ? "active" : "alert"}
                    />
                  </div>

                  <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-1 font-mono text-[12px]">
                    <div className="flex items-center justify-between text-[var(--text-muted)]">
                      <span>Transport:</span>
                      <span className="text-[var(--text)] font-medium">
                        {isLiveGui ? "Unix socket (SO_PEERCRED)" : "Stdio MCP pipe"}
                      </span>
                    </div>
                    {app.latencyMs !== null && (
                      <div className="flex items-center justify-between text-[var(--text-muted)]">
                        <span>Round-trip ping:</span>
                        <span className="text-[var(--accent-blue)] tabular-nums font-semibold">
                          {app.latencyMs.toFixed(2)} ms
                        </span>
                      </div>
                    )}
                    {app.socketPath && (
                      <div className="text-[var(--text-muted)] truncate" title={app.socketPath}>
                        Socket: {app.socketPath}
                      </div>
                    )}
                  </div>

                  <div className="space-y-2">
                    <span className="block text-[12px] font-medium text-[var(--text)]">
                      Exposed MCP capabilities
                    </span>
                    <div className="flex flex-wrap gap-1.5">
                      {Object.keys(app.capabilities).length > 0 ? (
                        Object.keys(app.capabilities).map((cap) => (
                          <span
                            key={cap}
                            className="inline-flex items-center rounded-[4px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-0.5 font-mono text-[12px] text-[var(--text)]"
                          >
                            {cap}
                          </span>
                        ))
                      ) : (
                        <span className="text-[12px] text-[var(--text-muted)]">Standard tools</span>
                      )}
                    </div>
                  </div>
                </div>

                <div className="pt-3 border-t border-[var(--border)] flex items-center gap-2">
                  <LinearButton
                    variant="secondary"
                    size="sm"
                    onClick={() => handlePing(app.appId)}
                    disabled={pingingAppId === app.appId || isOffline}
                    className="flex-1"
                  >
                    <Wifi className="size-3.5" strokeWidth={1.75} />
                    <span>Ping socket</span>
                  </LinearButton>

                  <LinearButton
                    variant={isLiveGui ? "secondary" : "primary"}
                    size="sm"
                    onClick={() => handleLaunch(app.appId)}
                    disabled={launchingAppId === app.appId}
                    className="flex-1"
                  >
                    <Play className="size-3.5" strokeWidth={1.75} />
                    <span>
                      {launchingAppId === app.appId
                        ? "Connecting"
                        : isLiveGui
                        ? "Focus GUI"
                        : "Launch GUI"}
                    </span>
                  </LinearButton>
                </div>
              </LinearCard>
            );
          })}
        </div>
      )}
    </div>
  );
};
