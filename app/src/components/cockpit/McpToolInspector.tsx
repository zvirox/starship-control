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
    <div className="flex-1 flex overflow-hidden">
      {/* Left Tools Explorer (360px) */}
      <aside className="w-[360px] border-r border-white/10 bg-[#0E131B]/50 flex flex-col p-4 space-y-4 select-none">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Wrench className="h-4 w-4 text-cyan-400" />
            <span className="font-bold text-sm tracking-wide text-slate-100">
              MCP Tool Catalog
            </span>
          </div>
          <button
            onClick={fetchTools}
            className="p-1 rounded-md bg-white/5 hover:bg-white/10 text-slate-400 hover:text-cyan-300"
            title="Refresh tools"
          >
            <RotateCw className="h-3 w-3" />
          </button>
        </div>

        {/* Search & Filters */}
        <div className="space-y-2">
          <div className="relative">
            <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-slate-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Filter MCP tools..."
              className="w-full rounded-lg border border-white/10 bg-black/30 pl-8 pr-3 py-1.5 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400"
            />
          </div>

          <div className="flex items-center gap-1 overflow-x-auto pb-1 text-[11px] font-mono">
            {["all", "eep", "ssp", "alp", "nwp", "atp"].map((app) => (
              <button
                key={app}
                onClick={() => setActiveAppFilter(app)}
                className={`px-2 py-0.5 rounded-md border uppercase transition-colors ${
                  activeAppFilter === app
                    ? "border-cyan-500/40 bg-cyan-500/20 text-cyan-300 font-semibold"
                    : "border-white/5 bg-white/5 text-slate-400 hover:text-slate-200"
                }`}
              >
                {app}
              </button>
            ))}
          </div>
        </div>

        {/* Tools List */}
        <div className="flex-1 overflow-y-auto space-y-2 pr-1">
          {loading ? (
            <div className="p-4 text-center text-xs text-slate-500">Loading tool schemas...</div>
          ) : (
            filteredTools.map((t) => {
              const isSelected = selectedTool?.name === t.name;
              return (
                <div
                  key={t.name}
                  onClick={() => handleSelectTool(t)}
                  className={`p-2.5 rounded-xl border text-xs cursor-pointer transition-all ${
                    isSelected
                      ? "border-cyan-500/50 bg-cyan-500/10 shadow-[0_0_12px_rgba(56,189,248,0.15)]"
                      : "border-white/5 bg-white/5 hover:border-white/15"
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="font-semibold text-slate-200 truncate">{t.name}</span>
                    {t.riskTier === 1 ? (
                      <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[9px] font-mono border border-emerald-500/30 bg-emerald-500/15 text-emerald-300">
                        <ShieldCheck className="h-2.5 w-2.5" />
                        <span>Tier 1</span>
                      </span>
                    ) : t.riskTier === 2 ? (
                      <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[9px] font-mono border border-amber-500/30 bg-amber-500/15 text-amber-300">
                        <AlertTriangle className="h-2.5 w-2.5" />
                        <span>Tier 2</span>
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[9px] font-mono border border-rose-500/30 bg-rose-500/15 text-rose-300">
                        <Flame className="h-2.5 w-2.5" />
                        <span>Tier 3</span>
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] text-slate-400 line-clamp-2 leading-relaxed">
                    {t.description}
                  </p>
                  <div className="mt-1 text-[10px] font-mono text-slate-500 flex items-center justify-between">
                    <span>{t.appName}</span>
                    <span>appId: {t.appId}</span>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </aside>

      {/* Right Detail & Test Dispatcher */}
      <main className="flex-1 flex flex-col p-6 overflow-y-auto space-y-6">
        {selectedTool ? (
          <>
            {/* Header */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-xl font-bold tracking-tight text-slate-100 font-mono">
                    {selectedTool.name}
                  </h2>
                  <p className="text-xs text-slate-400 mt-0.5">
                    Provided by <span className="text-cyan-300 font-semibold">{selectedTool.appName}</span> ({selectedTool.appId})
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  {selectedTool.riskTier === 1 ? (
                    <div className="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold border border-emerald-500/30 bg-emerald-500/15 text-emerald-300">
                      <ShieldCheck className="h-3.5 w-3.5" />
                      <span>Tier 1 · Safe / Autonomous</span>
                    </div>
                  ) : selectedTool.riskTier === 2 ? (
                    <div className="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold border border-amber-500/30 bg-amber-500/15 text-amber-300">
                      <AlertTriangle className="h-3.5 w-3.5" />
                      <span>Tier 2 · Mutating / Requires Approval</span>
                    </div>
                  ) : (
                    <div className="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold border border-rose-500/30 bg-rose-500/15 text-rose-300">
                      <Flame className="h-3.5 w-3.5" />
                      <span>Tier 3 · Destructive / Signed Token Required</span>
                    </div>
                  )}
                </div>
              </div>

              <p className="text-xs text-slate-300 leading-relaxed max-w-3xl">
                {selectedTool.description}
              </p>
            </div>

            {/* Parameter Schema */}
            <div className="space-y-2">
              <div className="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wider text-slate-400">
                <Code2 className="h-3.5 w-3.5 text-cyan-400" />
                <span>JSON Schema (Parameters)</span>
              </div>
              <pre className="rounded-xl border border-white/10 bg-black/40 p-3 text-xs font-mono text-cyan-300/90 overflow-x-auto">
                {JSON.stringify(selectedTool.parameters, null, 2)}
              </pre>
            </div>

            {/* Test Invocation Sandbox */}
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs">
                <div className="flex items-center gap-1.5 font-semibold uppercase tracking-wider text-slate-400">
                  <Terminal className="h-3.5 w-3.5 text-cyan-400" />
                  <span>Interactive Test Dispatcher (Hybrid Router)</span>
                </div>
                <span className="text-[10px] font-mono text-slate-500">
                  Transports: Live Socket $\to$ Stdio Child $\to$ Simulation
                </span>
              </div>

              <div className="space-y-2">
                <textarea
                  value={callArgs}
                  onChange={(e) => setCallArgs(e.target.value)}
                  rows={4}
                  className="w-full rounded-xl border border-white/10 bg-black/30 p-3 font-mono text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400 resize-none"
                  placeholder="Enter JSON arguments..."
                />

                <div className="flex items-center justify-between">
                  <button
                    onClick={handleExecute}
                    disabled={executing}
                    className="flex items-center gap-2 rounded-xl bg-cyan-400 hover:bg-cyan-300 px-4 py-2 text-xs font-semibold text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.35)] transition-all active:scale-95 disabled:opacity-50"
                  >
                    <Play className="h-3.5 w-3.5 fill-current" />
                    <span>{executing ? "Dispatching via MCP..." : "Execute Tool Call"}</span>
                  </button>

                  <span className="text-[10px] font-mono text-slate-500">
                    leaseTimeout: 10,000ms
                  </span>
                </div>
              </div>

              {/* Execution Feedback */}
              {executionResult && (
                <div className="space-y-1.5 pt-2">
                  <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-semibold font-mono">
                    <CheckCircle2 className="h-3.5 w-3.5" />
                    <span>Execution Result (Success):</span>
                  </div>
                  <pre className="rounded-xl border border-emerald-500/30 bg-emerald-950/20 p-3 text-xs font-mono text-emerald-200 overflow-x-auto">
                    {executionResult}
                  </pre>
                </div>
              )}

              {executionError && (
                <div className="space-y-1.5 pt-2">
                  <div className="flex items-center gap-1.5 text-xs text-rose-400 font-semibold font-mono">
                    <AlertTriangle className="h-3.5 w-3.5" />
                    <span>Execution Error:</span>
                  </div>
                  <pre className="rounded-xl border border-rose-500/30 bg-rose-950/20 p-3 text-xs font-mono text-rose-200 overflow-x-auto">
                    {executionError}
                  </pre>
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-slate-500 text-xs">
            Select a tool from the catalog to inspect schema and test dispatch.
          </div>
        )}
      </main>
    </div>
  );
};
