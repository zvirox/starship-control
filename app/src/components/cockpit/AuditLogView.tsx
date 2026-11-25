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
import { LinearCard, LinearButton } from "../linear";
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
    <div className="flex-1 flex flex-col p-5 overflow-hidden bg-[var(--bg-app)]">
      <div className="flex items-center justify-between pb-4 border-b border-[var(--border)]">
        <div className="flex items-center gap-3">
          <div className="flex size-9 items-center justify-center rounded-[6px] bg-[var(--bg-surface)] text-[var(--text-muted)] border border-[var(--border)]">
            <FileText className="size-4" />
          </div>
          <div>
            <h2 className="font-sans text-[16px] font-semibold text-[var(--text)] flex items-center gap-2">
              <span>Security and action audit trail</span>
              <span className="text-[12px] font-mono text-[var(--text-muted)] bg-[var(--bg-surface-2)] px-2 py-0.5 rounded-[4px] border border-[var(--border)]">
                Append-only WAL
              </span>
            </h2>
            <p className="font-sans text-[12px] text-[var(--text-muted)] mt-0.5">
              Immutable cryptographic ledger recording all tool invocations, approval gates, and human signatures.
            </p>
          </div>
        </div>

        <LinearButton variant="secondary" size="sm" onClick={fetchLogs} disabled={loading}>
          <RotateCcw className={`size-3.5 ${loading ? "animate-spin" : ""}`} />
          <span>Refresh</span>
        </LinearButton>
      </div>

      <div className="flex items-center justify-between gap-4 py-3">
        <div className="flex items-center gap-2 flex-1 max-w-md">
          <div className="relative w-full">
            <Search className="absolute left-2.5 top-2.5 size-3.5 text-[var(--text-muted)]" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => {
                const val = e.target.value;
                startTransition(() => setSearchQuery(val));
              }}
              placeholder="Filter by app, action, payload or signature token..."
              className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] pl-8 pr-3 py-1.5 font-sans text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
            />
          </div>
        </div>

        <div className="flex items-center gap-2 text-[12px] font-sans">
          <Filter className="size-3.5 text-[var(--text-muted)]" />
          <span className="text-[var(--text-muted)]">Filter tier</span>
          {["all", "1", "2", "3"].map((tier) => (
            <button
              key={tier}
              type="button"
              onClick={() => setFilterTier(tier)}
              className={`rounded-[4px] px-2 py-1 text-[12px] font-mono transition-colors cursor-pointer ${
                filterTier === tier
                  ? "bg-[var(--tint-blue)] text-[var(--accent-blue)] border border-[var(--accent-blue)] font-medium"
                  : "bg-[var(--bg-surface)] text-[var(--text-muted)] hover:text-[var(--text)] border border-[var(--border)]"
              }`}
            >
              {tier === "all" ? "All" : `Tier ${tier}`}
            </button>
          ))}
        </div>
      </div>

      <LinearCard level={1} className="flex-1 overflow-y-auto p-0">
        <table className="w-full text-left border-collapse text-[12px]">
          <thead>
            <tr className="border-b border-[var(--border)] bg-[var(--bg-surface-2)] font-sans text-[12px] uppercase tracking-wider text-[var(--text-muted)]">
              <th className="py-2.5 px-3 font-medium">ID</th>
              <th className="py-2.5 px-3 font-medium">Time (UTC)</th>
              <th className="py-2.5 px-3 font-medium">App</th>
              <th className="py-2.5 px-3 font-medium">Action type</th>
              <th className="py-2.5 px-3 font-medium">Risk tier</th>
              <th className="py-2.5 px-3 font-medium">Signature token</th>
              <th className="py-2.5 px-3 font-medium">Status</th>
              <th className="py-2.5 px-3 font-medium">Payload preview</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[var(--border)] font-sans text-[var(--text-muted)]">
            {filteredLogs.length === 0 ? (
              <tr>
                <td colSpan={8} className="py-12 text-center text-[var(--text-muted)]">
                  {loading ? "Loading audit records..." : "No matching audit log records found"}
                </td>
              </tr>
            ) : (
              filteredLogs.map((log) => (
                <tr key={log.id} className="hover:bg-[var(--bg-surface-2)] transition-colors">
                  <td className="py-2.5 px-3 font-mono text-[var(--text-muted)]">#{log.id}</td>
                  <td className="py-2.5 px-3 font-mono text-[var(--text-muted)] tabular-nums">
                    {log.createdAt ? log.createdAt.replace("T", " ").replace("Z", "") : "Now"}
                  </td>
                  <td className="py-2.5 px-3 font-mono font-semibold text-[var(--text)]">
                    {log.appId}
                  </td>
                  <td className="py-2.5 px-3 font-semibold text-[var(--text)]">
                    {log.actionType}
                  </td>
                  <td className="py-2.5 px-3">
                    <span
                      className={`inline-block font-mono text-[12px] font-semibold px-2 py-0.5 rounded-[4px] border ${
                        log.riskTier === 3
                          ? "bg-[var(--tint-red)] text-[var(--accent-red)] border-[var(--accent-red)]"
                          : log.riskTier === 2
                          ? "bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--accent-amber)]"
                          : "bg-[var(--tint-green)] text-[var(--accent-green)] border-[var(--accent-green)]"
                      }`}
                    >
                      Tier {log.riskTier}
                    </span>
                  </td>
                  <td className="py-2.5 px-3 font-mono">
                    {log.signatureToken ? (
                      <span className="inline-flex items-center gap-1 text-[var(--accent-green)] bg-[var(--tint-green)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-green)]">
                        <KeyRound className="size-3.5" />
                        <span>{log.signatureToken.slice(0, 16)}...</span>
                      </span>
                    ) : (
                      <span className="text-[var(--text-faint)]">—</span>
                    )}
                  </td>
                  <td className="py-2.5 px-3 font-mono">
                    <span
                      className={`inline-flex items-center gap-1 font-semibold ${
                        log.status === "success"
                          ? "text-[var(--accent-green)]"
                          : log.status === "denied"
                          ? "text-[var(--accent-red)]"
                          : "text-[var(--accent-amber)]"
                      }`}
                    >
                      {log.status === "success" ? (
                        <ShieldCheck className="size-3.5" />
                      ) : (
                        <ShieldAlert className="size-3.5" />
                      )}
                      <span className="capitalize">{log.status}</span>
                    </span>
                  </td>
                  <td className="py-2.5 px-3 font-mono text-[var(--text-muted)] max-w-xs truncate">
                    {log.payloadJson}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </LinearCard>
    </div>
  );
};
