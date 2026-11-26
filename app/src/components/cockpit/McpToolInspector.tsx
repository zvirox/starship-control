import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Wrench,
  Search,
  Play,
  CheckCircle2,
  AlertTriangle,
  Flame,
  ShieldCheck,
  RotateCw,
  Terminal,
  Code2,
} from "lucide-react";
import type { AggregatedTool, CallToolPayload } from "../../types/ipc";

export const McpToolInspector: React.FC = () => {
  const [tools, setTools] = useState<AggregatedTool[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [activeAppFilter, setActiveAppFilter] = useState<string>("all");
  const [selectedTool, setSelectedTool] = useState<AggregatedTool | null>(null);
  const [callArgs, setCallArgs] = useState<string>("{}");
  const [executing, setExecuting] = useState<boolean>(false);
  const [executionResult, setExecutionResult] = useState<string | null>(null);
  const [executionError, setExecutionError] = useState<string | null>(null);

  const fetchTools = async () => {
    setLoading(true);
    try {
      const res = await invoke<AggregatedTool[]>("cmd_mcp_list_tools");
      setTools(res);
      if (res.length > 0 && !selectedTool) {
        setSelectedTool(res[0]);
        setCallArgs(JSON.stringify(getSampleArguments(res[0]), null, 2));
      }
    } catch (e) {
      console.warn("Failed to list MCP tools:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTools();
  }, []);

  const getSampleArguments = (tool: AggregatedTool) => {
    if (tool.name.includes("cut_interval")) {
      return { startMs: 2500, endMs: 7800 };
    }
    if (tool.name.includes("auto_trim")) {
      return { thresholdDb: -35.0, minDurationMs: 600 };
    }
    if (tool.name.includes("cleanup_preset")) {
      return { preset: "voice_clean", trackId: 1 };
    }
    if (tool.name.includes("clean_package")) {
      return { aggressive: false };
    }
    return {};
  };

  const handleSelectTool = (tool: AggregatedTool) => {
    setSelectedTool(tool);
    setCallArgs(JSON.stringify(getSampleArguments(tool), null, 2));
    setExecutionResult(null);
    setExecutionError(null);
  };

  const handleExecute = async () => {
    if (!selectedTool) return;
    setExecuting(true);
    setExecutionResult(null);
    setExecutionError(null);

    let parsedArgs: Record<string, unknown> = {};
    try {
      parsedArgs = JSON.parse(callArgs);
    } catch {
      setExecutionError("Invalid JSON in tool arguments");
      setExecuting(false);
      return;
    }

    try {
      const payload: CallToolPayload = {
        appId: selectedTool.appId,
        toolName: selectedTool.name,
        arguments: parsedArgs,
        leaseTimeoutMs: 10000,
      };
      const res = await invoke<unknown>("cmd_mcp_call_tool", { payload });
      setExecutionResult(JSON.stringify(res, null, 2));
    } catch (err: unknown) {
      console.error("Tool execution failed:", err);
      setExecutionError(
        typeof err === "object" && err !== null && "userMessage" in err
          ? (err as { userMessage: string }).userMessage
          : "Tool execution failed"
      );
    } finally {
      setExecuting(false);
    }
  };

  const filteredTools = tools.filter((t) => {
    const matchesSearch =
      t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.appName.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesApp = activeAppFilter === "all" || t.appId === activeAppFilter;
    return matchesSearch && matchesApp;
  });

  return (
    <div className="flex-1 flex overflow-hidden bg-[var(--bg-app)] font-sans">
      <aside className="w-[340px] border-r border-[var(--border)] bg-[var(--bg-surface)] flex flex-col p-4 gap-4 select-none">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Wrench className="size-4 text-[var(--accent-blue)]" strokeWidth={1.75} />
            <span className="text-[14px] font-semibold text-[var(--text)]">MCP tool catalog</span>
          </div>
          <button
            type="button"
            onClick={fetchTools}
            className="p-1.5 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] hover:border-[var(--border-strong)] text-[var(--text-muted)] hover:text-[var(--text)] transition-colors focus-ring cursor-pointer"
            title="Refresh tools"
          >
            <RotateCw className="size-3.5" strokeWidth={1.75} />
          </button>
        </div>

        <div className="space-y-2">
          <div className="relative">
            <Search
              className="absolute left-3 top-2.5 size-3.5 text-[var(--text-muted)]"
              strokeWidth={1.75}
            />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Filter tools by name or description..."
              className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] pl-9 pr-3 py-1.5 text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
            />
          </div>

          <div className="flex items-center gap-1 overflow-x-auto pb-1 text-[12px] font-mono">
            {["all", "eep", "ssp", "alp", "nwp", "atp"].map((app) => (
              <button
                key={app}
                type="button"
                onClick={() => setActiveAppFilter(app)}
                className={`px-2 py-1 rounded-[4px] border transition-colors focus-ring cursor-pointer ${
                  activeAppFilter === app
                    ? "border-[var(--accent-blue)] bg-[var(--tint-blue)] text-[var(--accent-blue)] font-semibold"
                    : "border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:text-[var(--text)]"
                }`}
              >
                {app === "all" ? "All apps" : app.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto space-y-2 pr-1">
          {loading ? (
            <div className="p-4 text-center text-[12px] text-[var(--text-muted)]">Loading tool schemas...</div>
          ) : (
            filteredTools.map((t) => {
              const isSelected = selectedTool?.name === t.name;
              return (
                <div
                  key={t.name}
                  onClick={() => handleSelectTool(t)}
                  className={`p-3 rounded-[8px] border text-[12px] cursor-pointer transition-colors ${
                    isSelected
                      ? "border-[var(--accent-blue)] bg-[var(--tint-blue)]"
                      : "border-[var(--border)] bg-[var(--bg-surface-2)] hover:border-[var(--border-strong)]"
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="font-semibold text-[var(--text)] truncate">{t.name}</span>
                    {t.riskTier === 1 ? (
                      <span className="inline-flex items-center gap-1 rounded-[4px] border border-[var(--accent-green)] bg-[var(--tint-green)] px-2 py-0.5 font-mono text-[12px] font-semibold text-[var(--accent-green)]">
                        <ShieldCheck className="size-3.5" strokeWidth={1.75} />
                        <span>Tier 1</span>
                      </span>
                    ) : t.riskTier === 2 ? (
                      <span className="inline-flex items-center gap-1 rounded-[4px] border border-[var(--accent-amber)] bg-[var(--tint-amber)] px-2 py-0.5 font-mono text-[12px] font-semibold text-[var(--accent-amber)]">
                        <AlertTriangle className="size-3.5" strokeWidth={1.75} />
                        <span>Tier 2</span>
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 rounded-[4px] border border-[var(--accent-red)] bg-[var(--tint-red)] px-2 py-0.5 font-mono text-[12px] font-semibold text-[var(--accent-red)]">
                        <Flame className="size-3.5" strokeWidth={1.75} />
                        <span>Tier 3</span>
                      </span>
                    )}
                  </div>
                  <p className="text-[12px] text-[var(--text-muted)] line-clamp-2 leading-relaxed">
                    {t.description}
                  </p>
                  <div className="mt-1.5 text-[12px] font-mono text-[var(--text-muted)] flex items-center justify-between">
                    <span className="text-[var(--text)]">{t.appName}</span>
                    <span>appId: {t.appId}</span>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </aside>

      <main className="flex-1 flex flex-col p-5 overflow-y-auto gap-5">
        {selectedTool ? (
          <>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <h2 className="text-[16px] font-semibold text-[var(--text)] font-mono">
                    {selectedTool.name}
                  </h2>
                  <p className="text-[12px] text-[var(--text-muted)]">
                    Provided by{" "}
                    <span className="text-[var(--text)] font-medium">{selectedTool.appName}</span> (
                    {selectedTool.appId})
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  {selectedTool.riskTier === 1 ? (
                    <span className="inline-flex items-center gap-1.5 rounded-[4px] border border-[var(--border)] bg-[var(--bg-surface)] px-2.5 py-1 text-[12px] font-medium text-[var(--accent-green)]">
                      <ShieldCheck className="size-3.5" strokeWidth={1.75} />
                      <span>Tier 1 · Safe</span>
                    </span>
                  ) : selectedTool.riskTier === 2 ? (
                    <span className="inline-flex items-center gap-1.5 rounded-[4px] border border-[var(--border)] bg-[var(--bg-surface)] px-2.5 py-1 text-[12px] font-medium text-[var(--accent-amber)]">
                      <AlertTriangle className="size-3.5" strokeWidth={1.75} />
                      <span>Tier 2 · Requires approval</span>
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1.5 rounded-[4px] border border-[var(--border)] bg-[var(--bg-surface)] px-2.5 py-1 text-[12px] font-medium text-[var(--accent-red)]">
                      <Flame className="size-3.5" strokeWidth={1.75} />
                      <span>Tier 3 · Signed token required</span>
                    </span>
                  )}
                </div>
              </div>

              <p className="text-[13px] text-[var(--text-muted)] leading-relaxed max-w-3xl">
                {selectedTool.description}
              </p>
            </div>

            <div className="space-y-2">
              <div className="flex items-center gap-1.5 text-[12px] font-medium text-[var(--text-muted)]">
                <Code2 className="size-3.5" strokeWidth={1.75} />
                <span>JSON schema (parameters)</span>
              </div>
              <pre className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-[12px] font-mono text-[var(--text-muted)] overflow-x-auto">
                {JSON.stringify(selectedTool.parameters, null, 2)}
              </pre>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-[12px]">
                <div className="flex items-center gap-1.5 font-semibold text-[var(--text)]">
                  <Terminal className="size-3.5 text-[var(--accent-blue)]" strokeWidth={1.75} />
                  <span>Test dispatcher</span>
                </div>
                <span className="text-[12px] font-mono text-[var(--text-muted)]">
                  Transports: live socket → stdio child
                </span>
              </div>

              <div className="space-y-2">
                <textarea
                  value={callArgs}
                  onChange={(e) => setCallArgs(e.target.value)}
                  rows={4}
                  className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 font-mono text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)] resize-none"
                  placeholder="Enter JSON arguments e.g. {}"
                />

                <div className="flex items-center justify-between">
                  <button
                    type="button"
                    onClick={handleExecute}
                    disabled={executing}
                    className="flex items-center gap-2 rounded-[6px] bg-[var(--btn-primary-bg)] px-4 py-2 text-[12px] font-semibold text-[var(--btn-primary-text)] transition-opacity hover:opacity-90 active:opacity-80 disabled:opacity-40 focus-ring cursor-pointer"
                  >
                    <Play className="size-3.5" strokeWidth={2} />
                    <span>{executing ? "Dispatching via MCP..." : "Execute tool call"}</span>
                  </button>

                  <span className="text-[12px] font-mono text-[var(--text-muted)]">
                    leaseTimeout: 10,000ms
                  </span>
                </div>
              </div>

              {executionResult && (
                <div className="space-y-2 pt-2">
                  <div className="flex items-center gap-1.5 text-[12px] font-semibold text-[var(--accent-green)] font-mono">
                    <CheckCircle2 className="size-3.5" strokeWidth={1.75} />
                    <span>Execution result</span>
                  </div>
                  <pre className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-[12px] font-mono text-[var(--text)] overflow-x-auto">
                    {executionResult}
                  </pre>
                </div>
              )}

              {executionError && (
                <div className="space-y-2 pt-2">
                  <div className="flex items-center gap-1.5 text-[12px] font-semibold text-[var(--accent-red)] font-mono">
                    <AlertTriangle className="size-3.5" strokeWidth={1.75} />
                    <span>Execution error</span>
                  </div>
                  <pre className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-[12px] font-mono text-[var(--accent-red)] overflow-x-auto">
                    {executionError}
                  </pre>
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-[var(--text-muted)] text-[13px]">
            Select a tool from the catalog to inspect its schema and test dispatch
          </div>
        )}
      </main>
    </div>
  );
};
