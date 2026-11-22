import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Check, Key, Shield, Cloud, AlertCircle } from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type { ProviderConfigPayload } from "../../types/ipc";

export interface ModelSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const ModelSettingsModal: React.FC<ModelSettingsModalProps> = ({ isOpen, onClose }) => {
  const [provider, setProvider] = useState<"anthropic" | "openai" | "gemini" | "openrouter">("anthropic");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("https://api.anthropic.com/v1");
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  if (!isOpen) return null;

  const handleProviderChange = (p: typeof provider) => {
    setProvider(p);
    setStatusMsg(null);
    if (p === "anthropic") setBaseUrl("https://api.anthropic.com/v1");
    if (p === "openai") setBaseUrl("https://api.openai.com/v1");
    if (p === "gemini") setBaseUrl("https://generativelanguage.googleapis.com/v1beta");
    if (p === "openrouter") setBaseUrl("https://openrouter.ai/api/v1");
  };

  const handleSave = async () => {
    setSaving(true);
    setStatusMsg(null);
    try {
      const payload: ProviderConfigPayload = {
        providerId: provider,
        displayName:
          provider === "anthropic"
            ? "Anthropic Claude"
            : provider === "openai"
            ? "OpenAI"
            : provider === "gemini"
            ? "Google Gemini"
            : "OpenRouter",
        baseUrl,
        apiKey: apiKey ? apiKey : undefined,
        isEnabled: true,
      };

      await invoke("cmd_provider_config_set", { payload });
      setStatusMsg("Configuration saved to encrypted SQLite database.");
      setApiKey("");
      setTimeout(() => {
        setStatusMsg(null);
      }, 3000);
    } catch (e) {
      setStatusMsg(`Failed to save configuration: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 font-sans select-none">
      <GlassPanel recipe="modal" className="w-full max-w-lg p-5 space-y-4">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-white/10 pb-3">
          <div className="flex items-center gap-2">
            <Cloud className="h-5 w-5 text-cyan-400" />
            <h3 className="font-bold text-sm text-slate-100">Cloud Provider & API Key Manager</h3>
          </div>
          <button
            onClick={onClose}
            className="flex h-6 w-6 items-center justify-center rounded-full bg-white/5 hover:bg-white/10 text-slate-400 hover:text-slate-100"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* Provider Tabs */}
        <div className="grid grid-cols-4 gap-1.5 rounded-xl bg-black/40 p-1 border border-white/10 text-xs">
          {[
            { id: "anthropic", label: "Anthropic" },
            { id: "openai", label: "OpenAI" },
            { id: "gemini", label: "Gemini" },
            { id: "openrouter", label: "OpenRouter" },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => handleProviderChange(tab.id as typeof provider)}
              className={`rounded-lg py-1.5 font-medium transition-colors ${
                provider === tab.id
                  ? "bg-cyan-500/20 text-cyan-300 border border-cyan-400/40 shadow-[0_0_12px_rgba(56,189,248,0.2)]"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Input Fields */}
        <div className="space-y-3 text-xs">
          <div>
            <label className="block text-slate-400 mb-1 font-medium">Base Endpoint URL</label>
            <input
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              className="w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 font-mono text-xs text-slate-200 focus:outline-none focus:border-cyan-400"
            />
          </div>

          <div>
            <label className="block text-slate-400 mb-1 font-medium flex items-center justify-between">
              <span>API Key</span>
              <span className="text-[10px] text-slate-500 flex items-center gap-1">
                <Shield className="h-3 w-3 text-emerald-400" />
                <span>Protected (0600 SQLite)</span>
              </span>
            </label>
            <div className="relative">
              <Key className="absolute left-3 top-2.5 h-3.5 w-3.5 text-slate-500" />
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Paste API Key (sk-...)"
                className="w-full rounded-lg border border-white/10 bg-black/20 pl-9 pr-3 py-2 font-mono text-xs text-slate-200 focus:outline-none focus:border-cyan-400 placeholder:text-slate-600"
              />
            </div>
          </div>
        </div>

        {statusMsg && (
          <div className="rounded-lg bg-cyan-500/10 border border-cyan-400/30 p-2.5 text-xs text-cyan-300 flex items-center gap-2">
            <AlertCircle className="h-4 w-4 shrink-0" />
            <span>{statusMsg}</span>
          </div>
        )}

        {/* Footer */}
        <div className="flex items-center justify-between pt-2 border-t border-white/10">
          <span className="text-[10px] font-mono text-slate-500">
            Encrypted storage via provider_configs
          </span>
          <div className="flex items-center gap-2">
            <button
              onClick={onClose}
              className="px-3 py-1.5 rounded-lg border border-white/10 text-xs text-slate-400 hover:text-slate-200"
            >
              Cancel
            </button>
            <button
              disabled={saving}
              onClick={handleSave}
              className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-cyan-400 hover:bg-cyan-300 text-slate-950 font-semibold text-xs shadow-[0_0_12px_rgba(56,189,248,0.3)] transition-transform active:scale-95 disabled:opacity-50"
            >
              <Check className="h-3.5 w-3.5" />
              <span>{saving ? "Saving..." : "Save Configuration"}</span>
            </button>
          </div>
        </div>
      </GlassPanel>
    </div>
  );
};
