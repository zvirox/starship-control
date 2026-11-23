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
    <div className="flex-1 flex flex-col p-6 overflow-hidden bg-canvas">
      {/* Header */}
      <div className="flex items-center justify-between pb-4 border-b border-border-subtle">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-white/[0.03] text-text-secondary border border-border-subtle">
            <FileText className="h-4 w-4" />
          </div>
          <div>
            <h2 className="font-sans text-base font-medium text-text-primary flex items-center gap-2">
              <span>Security and action audit trail</span>
              <span className="text-[11px] font-mono text-text-disabled bg-white/[0.03] px-2 py-0.5 rounded-lg border border-border-subtle">
                Append-only WAL
              </span>
            </h2>
            <p className="font-sans text-xs text-text-secondary">
              Immutable cryptographic ledger recording all tool invocations, approval gates, and human signatures (DATABASE.md §3.8).
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={fetchLogs}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-xl border border-border-subtle bg-white/[0.03] px-3 py-2 text-xs font-medium text-text-secondary hover:bg-white/[0.06] hover:text-text-primary transition-colors"
        >
          <RotateCcw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex items-center justify-between gap-4 py-3">
        <div className="flex items-center gap-2 flex-1 max-w-md">
          <div className="relative w-full">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-text-disabled" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => {
                const val = e.target.value;
                startTransition(() => setSearchQuery(val));
              }}
              placeholder="Filter by app, action, payload or signature token"
              className="w-full rounded-xl border border-border-subtle bg-black/40 pl-8 pr-3 py-2 font-sans text-xs text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-accent/40"
            />
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs font-sans">
          <Filter className="h-3.5 w-3.5 text-text-secondary" />
          <span className="text-text-secondary">Filter tier</span>
          {["all", "1", "2", "3"].map((tier) => (
            <button
              key={tier}
              type="button"
              onClick={() => setFilterTier(tier)}
              className={`rounded-lg px-2 py-1 text-xs font-mono transition-colors ${
                filterTier === tier
                  ? "bg-accent-muted text-accent border border-accent/30"
                  : "bg-white/[0.03] text-text-secondary hover:text-text-primary border border-transparent"
              }`}
            >
              {tier === "all" ? "All" : `Tier ${tier}`}
            </button>
          ))}
        </div>
      </div>

      {/* Table Log Records */}
      <GlassPanel recipe="pane" className="flex-1 overflow-y-auto rounded-2xl border border-border-subtle p-0">
        <table className="w-full text-left border-collapse text-xs">
          <thead>
            <tr className="border-b border-border-subtle bg-white/[0.03] font-mono text-[11px] text-text-secondary">
              <th className="py-2 px-3">ID</th>
              <th className="py-2 px-3">Time (UTC)</th>
              <th className="py-2 px-3">App</th>
              <th className="py-2 px-3">Action type</th>
              <th className="py-2 px-3">Risk tier</th>
              <th className="py-2 px-3">Signature token</th>
              <th className="py-2 px-3">Status</th>
              <th className="py-2 px-3">Payload preview</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-subtle font-sans text-text-secondary">
            {filteredLogs.length === 0 ? (
              <tr>
                <td colSpan={8} className="py-12 text-center text-text-disabled font-sans">
                  {loading ? "Loading audit records" : "No matching audit log records found"}
                </td>
              </tr>
            ) : (
              filteredLogs.map((log) => (
                <tr key={log.id} className="hover:bg-white/[0.03] transition-colors">
                  <td className="py-2 px-3 font-mono text-text-disabled">#{log.id}</td>
                  <td className="py-2 px-3 font-mono text-text-disabled text-[11px] tabular">
                    {log.createdAt ? log.createdAt.replace("T", " ").replace("Z", "") : "Now"}
                  </td>
                  <td className="py-2 px-3 font-mono font-medium text-text-primary">
                    {log.appId}
                  </td>
                  <td className="py-2 px-3 font-medium text-text-primary">
                    {log.actionType}
                  </td>
                  <td className="py-2 px-3">
                    <span
                      className={`inline-block font-mono text-[11px] px-2 py-0.5 rounded-lg border ${
                        log.riskTier === 3
                          ? "bg-signal-gate-bg text-signal-gate border-signal-gate/30"
                          : log.riskTier === 2
                          ? "bg-signal-thinking-bg text-signal-thinking border-signal-thinking/30"
                          : "bg-signal-success-bg text-signal-success border-signal-success/30"
                      }`}
                    >
                      Tier {log.riskTier}
                    </span>
                  </td>
                  <td className="py-2 px-3 font-mono text-[11px]">
                    {log.signatureToken ? (
                      <span className="flex items-center gap-1 text-signal-success bg-signal-success-bg px-2 py-0.5 rounded-lg border border-signal-success/30">
                        <KeyRound className="h-3.5 w-3.5" />
                        <span>{log.signatureToken.slice(0, 16)}...</span>
                      </span>
                    ) : (
                      <span className="text-text-disabled">—</span>
                    )}
                  </td>
                  <td className="py-2 px-3 font-mono text-[11px]">
                    <span
                      className={`inline-flex items-center gap-1 font-medium ${
                        log.status === "success"
                          ? "text-signal-success"
                          : log.status === "denied"
                          ? "text-signal-gate"
                          : "text-signal-thinking"
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
                  <td className="py-2 px-3 font-mono text-[11px] text-text-disabled max-w-xs truncate">
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
