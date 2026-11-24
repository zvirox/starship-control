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

  const isStatusError = statusMsg?.startsWith("Failed") ?? false;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm font-sans select-none">
      <GlassPanel recipe="modal" className="w-full max-w-lg space-y-4 p-5">
        <div className="flex items-center justify-between border-b border-border-subtle pb-3">
          <div className="flex items-center gap-2">
            <Cloud className="h-4 w-4 text-text-secondary" strokeWidth={1.5} />
            <h3 className="text-sm font-medium text-text-primary">Cloud providers and API keys</h3>
          </div>
          <button
            onClick={onClose}
            title="Close"
            className="flex h-8 w-8 items-center justify-center rounded-xl text-text-secondary transition-colors duration-150 hover:bg-white/[0.05] hover:text-text-primary focus-ring"
          >
            <X className="h-4 w-4" strokeWidth={1.5} />
          </button>
        </div>

        <div className="grid grid-cols-4 gap-1 rounded-xl border border-border-subtle bg-black/20 p-1">
          {[
            { id: "anthropic", label: "Anthropic" },
            { id: "openai", label: "OpenAI" },
            { id: "gemini", label: "Gemini" },
            { id: "openrouter", label: "OpenRouter" },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => handleProviderChange(tab.id as typeof provider)}
              className={`rounded-lg py-1.5 text-xs transition-colors duration-150 focus-ring ${
                provider === tab.id
                  ? "border border-border-glass bg-surface-solid text-text-primary font-medium"
                  : "border border-transparent text-text-secondary hover:text-text-primary"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="space-y-3 text-xs">
          <div>
            <label className="mb-1 block text-text-secondary font-medium">Base endpoint URL</label>
            <input
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              className="w-full rounded-xl border border-border-subtle bg-surface-solid px-3 py-2 font-mono text-xs text-text-primary placeholder:text-text-secondary focus-ring focus:border-border-glass"
            />
          </div>

          <div>
            <label className="mb-1 flex items-center justify-between text-text-secondary font-medium">
              <span>API key</span>
              <span className="flex items-center gap-1 text-[11px] text-text-secondary font-medium">
                <Shield className="h-3.5 w-3.5 text-signal-success" strokeWidth={1.5} />
                <span>Protected (0600 SQLite)</span>
              </span>
            </label>
            <div className="relative">
              <Key className="absolute left-3 top-2.5 h-3.5 w-3.5 text-text-secondary" strokeWidth={1.5} />
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Paste API key"
                className="w-full rounded-xl border border-border-subtle bg-surface-solid py-2 pl-9 pr-3 font-mono text-xs text-text-primary placeholder:text-text-secondary focus-ring focus:border-border-glass"
              />
            </div>
          </div>
        </div>

        {statusMsg && (
          <div
            className={`flex items-center gap-2 rounded-xl border p-3 text-xs ${
              isStatusError
                ? "border-signal-gate/30 bg-signal-gate-bg text-signal-gate"
                : "border-border-subtle bg-white/[0.03] text-text-secondary"
            }`}
          >
            <AlertCircle className="h-4 w-4 shrink-0" strokeWidth={1.5} />
            <span>{statusMsg}</span>
          </div>
        )}

        <div className="flex items-center justify-between border-t border-border-subtle pt-3">
          <span className="font-mono text-[11px] text-text-secondary font-medium">
            Encrypted storage via provider_configs
          </span>
          <div className="flex items-center gap-2">
            <button
              onClick={onClose}
              className="rounded-xl border border-border-subtle px-3 py-1.5 text-xs text-text-secondary transition-colors duration-150 hover:bg-white/[0.05] hover:text-text-primary focus-ring"
            >
              Cancel
            </button>
            <button
              disabled={saving}
              onClick={handleSave}
              className="flex items-center gap-2 rounded-xl bg-accent px-4 py-1.5 text-xs font-medium text-accent-text transition-[opacity,transform] duration-150 hover:bg-accent-hover active:scale-[0.98] focus-ring disabled:opacity-50"
            >
              <Check className="h-3.5 w-3.5" strokeWidth={1.5} />
              <span>{saving ? "Saving" : "Save configuration"}</span>
            </button>
          </div>
        </div>
      </GlassPanel>
    </div>
  );
};
