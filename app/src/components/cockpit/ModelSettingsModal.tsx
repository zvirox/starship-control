import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, Key, Shield, Cloud, AlertCircle } from "lucide-react";
import { LinearButton, LinearDialog } from "../linear";
import type {
  ProviderConfigPayload,
  ProviderInfo,
  ProviderTestResponse,
} from "../../types/ipc";

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
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [testResult, setTestResult] = useState<ProviderTestResponse | null>(null);
  const [verifiedAt, setVerifiedAt] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    invoke<ProviderInfo[]>("cmd_provider_list")
      .then(setProviders)
      .catch((e: unknown) => console.warn("Provider list failed:", e));
  }, [isOpen]);

  if (!isOpen) return null;

  const handleProviderChange = (p: typeof provider) => {
    setProvider(p);
    setStatusMsg(null);
    setTestResult(null);
    setVerifiedAt(null);
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
      setApiKey("");
      setProviders(await invoke<ProviderInfo[]>("cmd_provider_list"));
      setStatusMsg("Key saved ✓ — checking connection…");
      const result = await invoke<ProviderTestResponse>(
        "cmd_provider_test_connection",
        { payload: { providerId: provider } },
      );
      setTestResult(result);
      setVerifiedAt(new Date().toLocaleTimeString());
      setStatusMsg(
        result.ok
          ? `Key saved ✓ — ${result.modelCount} models loaded`
          : `Key saved ✓ — test failed: ${result.message}`,
      );
    } catch (e) {
      setStatusMsg(`Failed to save configuration: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const runTest = async () => {
    setTesting(true);
    try {
      const result = await invoke<ProviderTestResponse>(
        "cmd_provider_test_connection",
        { payload: { providerId: provider } },
      );
      setTestResult(result);
      setVerifiedAt(new Date().toLocaleTimeString());
      setStatusMsg(
        result.ok
          ? `${provider}: ${result.modelCount} models loaded (${result.latencyMs} ms)`
          : `${provider}: ${result.message}`,
      );
    } catch (e) {
      setStatusMsg(`Failed to test connection: ${String(e)}`);
    } finally {
      setTesting(false);
    }
  };

  const fetchModels = async () => {
    setTesting(true);
    try {
      const response = await invoke<{ modelCount?: number; models?: unknown[] }>(
        "cmd_provider_refresh_models",
        { payload: { providerId: provider } },
      );
      const count = response.models?.length ?? 0;
      setStatusMsg(`${provider}: ${count} models loaded`);
      setVerifiedAt(new Date().toLocaleTimeString());
    } catch (e) {
      setStatusMsg(`Failed to fetch models: ${String(e)}`);
    } finally {
      setTesting(false);
    }
  };

  const hasKey =
    providers.find((candidate) => candidate.providerId === provider)?.hasKey ?? false;

  const isStatusError = statusMsg?.startsWith("Failed") ?? false;

  return (
    <LinearDialog
      open={isOpen}
      onClose={onClose}
      title="Cloud providers and API keys"
      icon={<Cloud className="size-4 text-[var(--text-muted)]" strokeWidth={1.75} />}
      footer={
        <>
          <span className="font-mono text-[12px] text-[var(--text-muted)]">
            Protected SQLite storage (provider_configs)
          </span>
          <div className="flex items-center gap-2">
            <LinearButton variant="secondary" size="sm" onClick={onClose}>
              Cancel
            </LinearButton>
            <LinearButton
              variant="primary"
              size="sm"
              disabled={saving}
              onClick={handleSave}
            >
              <Check className="size-3.5" strokeWidth={1.75} />
              <span>{saving ? "Saving" : "Save configuration"}</span>
            </LinearButton>
          </div>
        </>
      }
    >
      <div className="space-y-4">
        <div className="grid grid-cols-4 gap-1 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-1">
          {[
            { id: "anthropic", label: "Anthropic" },
            { id: "openai", label: "OpenAI" },
            { id: "gemini", label: "Gemini" },
            { id: "openrouter", label: "OpenRouter" },
          ].map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => handleProviderChange(tab.id as typeof provider)}
              className={`rounded-[4px] py-1.5 text-[12px] transition-colors duration-150 focus-ring cursor-pointer ${
                provider === tab.id
                  ? "border border-[var(--border-strong)] bg-[var(--bg-surface)] text-[var(--text)] font-medium"
                  : "border border-transparent text-[var(--text-muted)] hover:text-[var(--text)]"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="flex flex-wrap items-center justify-between gap-2 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-3 py-2 text-[12px]">
          <span className="text-[var(--text-muted)]">
            {hasKey ? "Key saved ✓" : "No key stored"} · last verified{" "}
            {verifiedAt ?? "never"}
          </span>
          <div className="flex items-center gap-2">
            <LinearButton variant="secondary" size="sm" disabled={testing} onClick={runTest}>
              Test connection
            </LinearButton>
            <LinearButton variant="secondary" size="sm" disabled={testing || !hasKey} onClick={fetchModels}>
              Fetch models
            </LinearButton>
          </div>
        </div>
        {testResult && (
          <div className="text-[12px] text-[var(--text-muted)]">
            {testResult.ok
              ? `${testResult.modelCount} models · ${testResult.latencyMs} ms`
              : `Failed: ${testResult.errorCode ?? testResult.message}`}
          </div>
        )}

        <div className="space-y-3 text-[12px]">
          <div>
            <label className="mb-1 block text-[var(--text-muted)] font-medium">
              Base endpoint URL
            </label>
            <input
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-3 py-2 font-mono text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
            />
          </div>

          <div>
            <label className="mb-1 flex items-center justify-between text-[var(--text-muted)] font-medium">
              <span>API key</span>
              <span className="flex items-center gap-1 text-[12px] text-[var(--text-muted)]">
                <Shield className="size-3.5 text-[var(--accent-green)]" strokeWidth={1.75} />
                <span>Protected (0600 SQLite)</span>
              </span>
            </label>
            <div className="relative">
              <Key className="absolute left-3 top-2.5 size-3.5 text-[var(--text-muted)]" strokeWidth={1.75} />
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Paste API key"
                className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] py-2 pl-9 pr-3 font-mono text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
              />
            </div>
          </div>
        </div>

        {statusMsg && (
          <div
            className={`flex items-center gap-2 rounded-[6px] border p-3 text-[12px] ${
              isStatusError
                ? "border-[var(--accent-red)] bg-[var(--tint-red)] text-[var(--accent-red)]"
                : "border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)]"
            }`}
          >
            <AlertCircle className="size-4 shrink-0" strokeWidth={1.75} />
            <span>{statusMsg}</span>
          </div>
        )}
      </div>
    </LinearDialog>
  );
};
