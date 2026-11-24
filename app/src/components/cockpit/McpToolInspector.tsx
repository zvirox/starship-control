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
    <div className="flex-1 flex overflow-hidden font-sans">
      {/* Left Tools Explorer (360px) */}
      <aside className="w-[360px] border-r border-border-subtle bg-raised/50 flex flex-col p-4 gap-4 select-none">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Wrench className="h-4 w-4 text-accent" strokeWidth={1.5} />
            <span className="text-sm font-medium text-text-primary">MCP tool catalog</span>
          </div>
          <button
            onClick={fetchTools}
            className="p-1.5 rounded-lg bg-white/[0.03] hover:bg-white/10 text-text-secondary hover:text-text-primary transition-colors focus-visible:outline-none focus-visible:text-accent cursor-pointer"
            title="Refresh tools"
          >
            <RotateCw className="h-3.5 w-3.5" strokeWidth={1.5} />
          </button>
        </div>

        {/* Search & Filters */}
        <div className="space-y-2">
          <div className="relative">
            <Search
              className="absolute left-3 top-3 h-4 w-4 text-text-secondary"
              strokeWidth={1.5}
            />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Filter tools by name or description..."
              className="w-full rounded-xl border border-border-subtle bg-surface-solid pl-9 pr-3 py-2 text-xs font-medium text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent"
            />
          </div>

          <div className="flex items-center gap-1 overflow-x-auto pb-1 text-[11px] font-mono">
            {["all", "eep", "ssp", "alp", "nwp", "atp"].map((app) => (
              <button
                key={app}
                onClick={() => setActiveAppFilter(app)}
                className={`px-2 py-1 rounded-lg border transition-colors focus-visible:outline-none focus-visible:text-accent cursor-pointer ${
                  activeAppFilter === app
                    ? "border-accent/40 bg-accent-muted text-accent font-semibold"
                    : "border-border-subtle bg-white/[0.03] text-text-secondary hover:text-text-primary"
                }`}
              >
                {app === "all" ? "All apps" : app.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        {/* Tools List */}
        <div className="flex-1 overflow-y-auto space-y-2 pr-1">
          {loading ? (
            <div className="p-4 text-center text-xs font-medium text-text-secondary">Loading tool schemas...</div>
          ) : (
            filteredTools.map((t) => {
              const isSelected = selectedTool?.name === t.name;
              return (
                <div
                  key={t.name}
                  onClick={() => handleSelectTool(t)}
                  className={`p-3 rounded-xl border text-xs cursor-pointer transition-colors ${
                    isSelected
                      ? "border-accent/40 bg-accent-muted"
                      : "border-border-subtle bg-white/[0.03] hover:border-border-glass"
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="font-semibold text-text-primary truncate">{t.name}</span>
                    {t.riskTier === 1 ? (
                      <span className="inline-flex items-center gap-1 rounded-lg border border-signal-success/30 bg-signal-success-bg px-2 py-0.5 font-mono text-[11px] font-semibold text-signal-success">
                        <ShieldCheck className="h-3.5 w-3.5" strokeWidth={1.5} />
                        <span>Tier 1</span>
                      </span>
                    ) : t.riskTier === 2 ? (
                      <span className="inline-flex items-center gap-1 rounded-lg border border-signal-thinking/30 bg-signal-thinking-bg px-2 py-0.5 font-mono text-[11px] font-semibold text-signal-thinking">
                        <AlertTriangle className="h-3.5 w-3.5" strokeWidth={1.5} />
                        <span>Tier 2</span>
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 rounded-lg border border-signal-gate/30 bg-signal-gate-bg px-2 py-0.5 font-mono text-[11px] font-semibold text-signal-gate">
                        <Flame className="h-3.5 w-3.5" strokeWidth={1.5} />
                        <span>Tier 3</span>
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] text-text-secondary font-medium line-clamp-2 leading-relaxed">
                    {t.description}
                  </p>
                  <div className="mt-1.5 text-[11px] font-mono text-text-secondary font-medium flex items-center justify-between">
                    <span className="text-text-primary">{t.appName}</span>
                    <span>appId: {t.appId}</span>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </aside>

      {/* Right Detail & Test Dispatcher */}
      <main className="flex-1 flex flex-col p-6 overflow-y-auto gap-6">
        {selectedTool ? (
          <>
            {/* Header */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <h2 className="text-lg font-medium text-text-primary font-mono">
                    {selectedTool.name}
                  </h2>
                  <p className="text-xs text-text-secondary">
                    Provided by{" "}
                    <span className="text-text-primary font-medium">{selectedTool.appName}</span> (
                    {selectedTool.appId})
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  {selectedTool.riskTier === 1 ? (
                    <span className="inline-flex items-center gap-1.5 rounded-lg border border-border-subtle bg-white/[0.03] px-3 py-1.5 text-[11px] font-medium text-signal-success">
                      <ShieldCheck className="h-3.5 w-3.5" strokeWidth={1.5} />
                      <span>Tier 1 · Safe</span>
                    </span>
                  ) : selectedTool.riskTier === 2 ? (
                    <span className="inline-flex items-center gap-1.5 rounded-lg border border-border-subtle bg-white/[0.03] px-3 py-1.5 text-[11px] font-medium text-signal-thinking">
                      <AlertTriangle className="h-3.5 w-3.5" strokeWidth={1.5} />
                      <span>Tier 2 · Requires approval</span>
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1.5 rounded-lg border border-border-subtle bg-white/[0.03] px-3 py-1.5 text-[11px] font-medium text-signal-gate">
                      <Flame className="h-3.5 w-3.5" strokeWidth={1.5} />
                      <span>Tier 3 · Signed token required</span>
                    </span>
                  )}
                </div>
              </div>

              <p className="text-xs text-text-secondary leading-relaxed max-w-3xl">
                {selectedTool.description}
              </p>
            </div>

            {/* Parameter Schema */}
            <div className="space-y-2">
              <div className="flex items-center gap-1.5 text-xs font-medium text-text-secondary">
                <Code2 className="h-3.5 w-3.5 text-text-secondary" strokeWidth={1.5} />
                <span>JSON schema (parameters)</span>
              </div>
              <pre className="rounded-xl border border-border-subtle bg-black/30 p-3 text-[11px] font-mono text-text-secondary overflow-x-auto">
                {JSON.stringify(selectedTool.parameters, null, 2)}
              </pre>
            </div>

            {/* Test Invocation Sandbox */}
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs">
                <div className="flex items-center gap-1.5 font-semibold text-text-primary">
                  <Terminal className="h-3.5 w-3.5 text-accent" strokeWidth={1.5} />
                  <span>Test dispatcher</span>
                </div>
                <span className="text-[11px] font-mono text-text-secondary font-medium">
                  Transports: live socket → stdio child
                </span>
              </div>

              <div className="space-y-2">
                <textarea
                  value={callArgs}
                  onChange={(e) => setCallArgs(e.target.value)}
                  rows={4}
                  className="w-full rounded-xl border border-border-subtle bg-surface-solid p-3 font-mono text-[11px] text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent resize-none font-medium"
                  placeholder="Enter JSON arguments e.g. {}"
                />

                <div className="flex items-center justify-between">
                  <button
                    onClick={handleExecute}
                    disabled={executing}
                    className="flex items-center gap-2 rounded-xl bg-accent hover:bg-accent-hover px-4 py-2 text-xs font-semibold text-accent-text transition-colors disabled:opacity-40 focus-visible:outline-none cursor-pointer"
                  >
                    <Play className="h-3.5 w-3.5" strokeWidth={2} />
                    <span>{executing ? "Dispatching via MCP..." : "Execute tool call"}</span>
                  </button>

                  <span className="text-[11px] font-mono text-text-secondary font-medium">
                    leaseTimeout: 10,000ms
                  </span>
                </div>
              </div>

              {/* Execution Feedback */}
              {executionResult && (
                <div className="space-y-2 pt-2">
                  <div className="flex items-center gap-1.5 text-[11px] font-semibold text-signal-success font-mono">
                    <CheckCircle2 className="h-3.5 w-3.5" strokeWidth={1.5} />
                    <span>Execution result</span>
                  </div>
                  <pre className="rounded-xl border border-border-subtle bg-surface-solid p-3 text-[11px] font-mono text-text-primary overflow-x-auto">
                    {executionResult}
                  </pre>
                </div>
              )}

              {executionError && (
                <div className="space-y-2 pt-2">
                  <div className="flex items-center gap-1.5 text-[11px] font-semibold text-signal-gate font-mono">
                    <AlertTriangle className="h-3.5 w-3.5" strokeWidth={1.5} />
                    <span>Execution error</span>
                  </div>
                  <pre className="rounded-xl border border-border-subtle bg-surface-solid p-3 text-[11px] font-mono text-signal-gate overflow-x-auto">
                    {executionError}
                  </pre>
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-text-secondary font-medium text-xs">
            Select a tool from the catalog to inspect its schema and test dispatch
          </div>
        )}
      </main>
    </div>
  );
};
