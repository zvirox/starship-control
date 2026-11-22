import React, { useEffect, useState, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  FileText,
  ShieldAlert,
  ShieldCheck,
  RotateCcw,
  KeyRound,
  Filter,
  Search,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type { AuditRecord } from "../../types/ipc";

export const AuditLogView: React.FC = () => {
  const [logs, setLogs] = useState<AuditRecord[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [filterTier, setFilterTier] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [, startTransition] = useTransition();

  const fetchLogs = async () => {
    try {
      setLoading(true);
      const records = await invoke<AuditRecord[]>("cmd_audit_list_logs", {
        payload: { limit: 100, offset: 0 },
      });
      setLogs(records);
    } catch (err) {
      console.warn("Failed to fetch audit logs:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchLogs();
  }, []);

  const filteredLogs = logs.filter((log) => {
    if (filterTier !== "all" && log.riskTier.toString() !== filterTier) {
      return false;
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return (
        log.appId.toLowerCase().includes(q) ||
        log.actionType.toLowerCase().includes(q) ||
        log.payloadJson.toLowerCase().includes(q) ||
        (log.signatureToken && log.signatureToken.toLowerCase().includes(q))
      );
    }
    return true;
  });

  return (
    <div className="flex-1 flex flex-col p-6 overflow-hidden bg-[#080B10]">
      {/* Header */}
      <div className="flex items-center justify-between pb-4 border-b border-white/10">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-purple-500/20 text-purple-400 border border-purple-500/30">
            <FileText className="h-5 w-5" />
          </div>
          <div>
            <h2 className="font-sans text-base font-bold text-slate-100 flex items-center gap-2">
              <span>Security & Action Audit Trail</span>
              <span className="text-[11px] font-mono text-purple-300 bg-purple-950/60 px-2 py-0.5 rounded border border-purple-500/30">
                Append-Only WAL
              </span>
            </h2>
            <p className="font-sans text-xs text-slate-400">
              Immutable cryptographic ledger recording all tool invocations, approval gates, and human signatures (DATABASE.md §3.8).
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={fetchLogs}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-white/10 transition-colors"
        >
          <RotateCcw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex items-center justify-between gap-4 py-3">
        <div className="flex items-center gap-2 flex-1 max-w-md">
          <div className="relative w-full">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-slate-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => {
                const val = e.target.value;
                startTransition(() => setSearchQuery(val));
              }}
              placeholder="Filter by app, action, payload or signature token..."
              className="w-full rounded-xl border border-white/10 bg-black/40 pl-8 pr-3 py-1.5 font-sans text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-400"
            />
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs font-sans">
          <Filter className="h-3.5 w-3.5 text-slate-400" />
          <span className="text-slate-400">Filter Tier:</span>
          {["all", "1", "2", "3"].map((tier) => (
            <button
              key={tier}
              type="button"
              onClick={() => setFilterTier(tier)}
              className={`rounded-lg px-2.5 py-1 text-xs font-mono transition-colors ${
                filterTier === tier
                  ? "bg-cyan-500/20 text-cyan-300 border border-cyan-400/30"
                  : "bg-white/5 text-slate-400 hover:text-slate-200"
              }`}
            >
              {tier === "all" ? "All" : `Tier ${tier}`}
            </button>
          ))}
        </div>
      </div>

      {/* Table Log Records */}
      <GlassPanel recipe="pane" className="flex-1 overflow-y-auto rounded-2xl border border-white/10 p-0">
        <table className="w-full text-left border-collapse text-xs">
          <thead>
            <tr className="border-b border-white/10 bg-white/5 font-mono text-[11px] text-slate-400 uppercase tracking-wider">
              <th className="py-2.5 px-3"># ID</th>
              <th className="py-2.5 px-3">Time (UTC)</th>
              <th className="py-2.5 px-3">App</th>
              <th className="py-2.5 px-3">Action Type</th>
              <th className="py-2.5 px-3">Risk Tier</th>
              <th className="py-2.5 px-3">Signature Token</th>
              <th className="py-2.5 px-3">Status</th>
              <th className="py-2.5 px-3">Payload Preview</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-white/5 font-sans text-slate-300">
            {filteredLogs.length === 0 ? (
              <tr>
                <td colSpan={8} className="py-12 text-center text-slate-500 font-sans">
                  {loading ? "Loading audit records..." : "No matching audit log records found."}
                </td>
              </tr>
            ) : (
              filteredLogs.map((log) => (
                <tr key={log.id} className="hover:bg-white/5 transition-colors">
                  <td className="py-2.5 px-3 font-mono text-slate-400">#{log.id}</td>
                  <td className="py-2.5 px-3 font-mono text-slate-400 text-[11px]">
                    {log.createdAt ? log.createdAt.replace("T", " ").replace("Z", "") : "Now"}
                  </td>
                  <td className="py-2.5 px-3 font-mono font-bold text-cyan-300 uppercase">
                    {log.appId}
                  </td>
                  <td className="py-2.5 px-3 font-medium text-slate-200">
                    {log.actionType}
                  </td>
                  <td className="py-2.5 px-3">
                    <span
                      className={`inline-block font-mono text-[10px] px-2 py-0.5 rounded-full border ${
                        log.riskTier === 3
                          ? "bg-rose-500/20 text-rose-300 border-rose-500/40"
                          : log.riskTier === 2
                          ? "bg-amber-500/20 text-amber-300 border-amber-500/40"
                          : "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                      }`}
                    >
                      Tier {log.riskTier}
                    </span>
                  </td>
                  <td className="py-2.5 px-3 font-mono text-[11px]">
                    {log.signatureToken ? (
                      <span className="flex items-center gap-1 text-emerald-300 bg-emerald-950/40 px-1.5 py-0.5 rounded border border-emerald-500/30">
                        <KeyRound className="h-2.5 w-2.5" />
                        <span>{log.signatureToken.slice(0, 16)}...</span>
                      </span>
                    ) : (
                      <span className="text-slate-600">—</span>
                    )}
                  </td>
                  <td className="py-2.5 px-3 font-mono text-[11px]">
                    <span
                      className={`inline-flex items-center gap-1 font-semibold ${
                        log.status === "success"
                          ? "text-emerald-400"
                          : log.status === "denied"
                          ? "text-rose-400"
                          : "text-amber-400"
                      }`}
                    >
                      {log.status === "success" ? (
                        <ShieldCheck className="h-3 w-3" />
                      ) : (
                        <ShieldAlert className="h-3 w-3" />
                      )}
                      <span className="capitalize">{log.status}</span>
                    </span>
                  </td>
                  <td className="py-2.5 px-3 font-mono text-[11px] text-slate-400 max-w-xs truncate">
                    {log.payloadJson}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </GlassPanel>
    </div>
  );
};
