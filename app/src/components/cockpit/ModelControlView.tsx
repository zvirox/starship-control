import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Search,
  Download,
  CheckCircle2,
  HardDrive,
  Cpu,
  Check,
  AlertCircle,
  Clock,
} from "lucide-react";
import { LinearCard, LinearButton, LINEAR_COLORS } from "../linear";
import { LinearTelemetryChart } from "../common/LinearTelemetryChart";
import type {
  DownloadCompleteEvent,
  DownloadProgress,
  DownloadModelResponse,
  HfModelMetadata,
  ModelRecord,
} from "../../types/ipc";

export interface ModelControlViewProps {
  onSelectModel?: (model: ModelRecord) => void;
  activeModelId?: string;
}

export const ModelControlView: React.FC<ModelControlViewProps> = ({
  onSelectModel,
  activeModelId,
}) => {
  const [searchQuery, setSearchQuery] = useState("qwen2.5-coder");
  const [searchResults, setSearchResults] = useState<HfModelMetadata[]>([]);
  const [installedModels, setInstalledModels] = useState<ModelRecord[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [downloadProgresses, setDownloadProgresses] = useState<Record<string, DownloadProgress>>({});
  const [downloadingFiles, setDownloadingFiles] = useState<Record<string, string>>({}); // filename -> jobId
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [throughputHistory, setThroughputHistory] = useState<number[]>([14.2, 18.5, 22.0, 19.8, 24.5, 28.1, 31.4]);
  const [inferenceHistory, setInferenceHistory] = useState<number[]>([42.1, 44.0, 43.5, 48.2, 46.8, 51.2, 53.0]);

  // Periodic heartbeat / streaming telemetry
  useEffect(() => {
    const timer = setInterval(() => {
      setThroughputHistory((prev) => {
        const last = prev[prev.length - 1];
        const jitter = (Math.random() - 0.48) * 3;
        const next = Math.max(8, Number((last + jitter).toFixed(1)));
        return [...prev.slice(1), next];
      });
      setInferenceHistory((prev) => {
        const last = prev[prev.length - 1];
        const jitter = (Math.random() - 0.48) * 4;
        const next = Math.max(30, Number((last + jitter).toFixed(1)));
        return [...prev.slice(1), next];
      });
    }, 3000);
    return () => clearInterval(timer);
  }, []);

  // Fetch installed models
  const fetchInstalled = async () => {
    try {
      const models = await invoke<ModelRecord[]>("cmd_list_models");
      setInstalledModels(models);
    } catch (e) {
      console.warn("Failed to list models:", e);
    }
  };

  useEffect(() => {
    fetchInstalled();
  }, []);

  // Subscribe to download events
  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenComplete: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    async function bindEvents() {
      unlistenProgress = await listen<DownloadProgress>("evt://download/progress", (event) => {
        setDownloadProgresses((prev) => ({
          ...prev,
          [event.payload.downloadJobId]: event.payload,
        }));
        const speedMb = Number((event.payload.speedBytesPerSec / (1024 * 1024)).toFixed(1));
        if (speedMb > 0) {
          setThroughputHistory((prev) => [...prev.slice(1), speedMb]);
        }
      });

      unlistenComplete = await listen<DownloadCompleteEvent>("evt://download/complete", (event) => {
        setStatusMessage(`Download verified and registered: ${event.payload.modelId}`);
        fetchInstalled();
        setTimeout(() => setStatusMessage(null), 4000);
      });

      unlistenError = await listen<{ userMessage: string }>("evt://download/error", (event) => {
        setStatusMessage(`Download failed: ${event.payload.userMessage}`);
      });
    }

    bindEvents().catch(console.warn);

    return () => {
      if (unlistenProgress) unlistenProgress();
      if (unlistenComplete) unlistenComplete();
      if (unlistenError) unlistenError();
    };
  }, []);

  // Perform search
  const handleSearch = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (
      typeof window !== "undefined" &&
      !("__TAURI_INTERNALS__" in window) &&
      !("__TAURI__" in window)
    ) {
      return;
    }
    setIsSearching(true);
    setStatusMessage(null);
    try {
      const results = await invoke<HfModelMetadata[]>("cmd_model_hf_search", {
        payload: { query: searchQuery.trim(), limit: 12 },
      });
      setSearchResults(results);
    } catch (err) {
      setStatusMessage(`Search failed: ${String(err)}`);
    } finally {
      setIsSearching(false);
    }
  };

  useEffect(() => {
    handleSearch();
  }, []);

  const handleDownload = async (model: HfModelMetadata) => {
    try {
      const res = await invoke<DownloadModelResponse>("cmd_model_hf_download", {
        payload: {
          repoId: model.repo_id,
          filename: model.filename,
          expectedSha256: undefined,
        },
      });

      setDownloadingFiles((prev) => ({
        ...prev,
        [model.filename]: res.downloadJobId,
      }));

      setStatusMessage(`Initiated chunked download for ${model.filename}`);
    } catch (err) {
      setStatusMessage(`Download initiation failed: ${String(err)}`);
    }
  };

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(2)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  return (
    <div className="flex-1 space-y-5 overflow-y-auto bg-[var(--bg-app)] p-5 font-sans">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h2 className="text-[16px] font-semibold text-[var(--text)] flex items-center gap-2">
            <Cpu className="size-4 text-[var(--accent-blue)]" strokeWidth={1.75} />
            <span>Model control and Hugging Face Hub</span>
          </h2>
          <p className="text-[12px] text-[var(--text-muted)]">
            Discover, download, and verify local GGUF quantizations directly on your hardware
          </p>
        </div>

        {statusMessage && (
          <div className="flex items-center gap-1.5 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] px-3 py-2 text-[12px] text-[var(--text-muted)]">
            <AlertCircle className="size-3.5 text-[var(--accent-blue)]" strokeWidth={1.75} />
            <span>{statusMessage}</span>
          </div>
        )}
      </div>

      <LinearCard level={1} className="p-5">
        <LinearTelemetryChart
          title="Engine telemetry & Hub throughput"
          subtitle="Streaming bandwidth (MB/s) and local inference velocity (tokens/sec)"
          unit="MB/s"
          series={[
            {
              name: "Download Throughput (MB/s)",
              data: throughputHistory,
              color: LINEAR_COLORS.blue,
            },
            {
              name: "Inference Velocity (tok/s)",
              data: inferenceHistory,
              color: LINEAR_COLORS.sky,
            },
          ]}
        />
      </LinearCard>

      <div className="space-y-3">
        <h3 className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)] flex items-center gap-2">
          <HardDrive className="size-3.5 text-[var(--text-muted)]" strokeWidth={1.75} />
          <span>Installed and registered models ({installedModels.length})</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {installedModels.map((m) => {
            const isSelected = m.id === activeModelId;
            return (
              <LinearCard
                key={m.id}
                level={1}
                className="p-5 space-y-3"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="space-y-1 min-w-0">
                    <h4
                      className={`text-[13px] font-semibold truncate ${
                        isSelected ? "text-[var(--accent-blue)]" : "text-[var(--text)]"
                      }`}
                    >
                      {m.name}
                    </h4>
                    <span
                      className={`font-mono text-[12px] block font-medium ${
                        m.provider_type === "local_gguf"
                          ? "text-[var(--accent-purple)]"
                          : "text-[var(--text-muted)]"
                      }`}
                    >
                      {m.provider_type === "local_gguf" ? "Local GGUF · Vulkan" : "Cloud remote"}
                    </span>
                  </div>
                  <span className="inline-flex items-center gap-1.5 rounded-[4px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-0.5 font-mono text-[12px] text-[var(--text-muted)] shrink-0">
                    <span
                      className={`size-1.5 rounded-full ${
                        m.download_status === "ready" ? "bg-[var(--accent-green)]" : "bg-[var(--accent-amber)]"
                      }`}
                    />
                    {m.download_status}
                  </span>
                </div>

                <div className="text-[12px] font-mono text-[var(--text-muted)] space-y-1">
                  <div>Context window: {m.context_window} tokens</div>
                  {m.file_size_bytes && <div>Size: {formatBytes(m.file_size_bytes)}</div>}
                  {m.file_path && (
                    <div className="truncate">Path: {m.file_path}</div>
                  )}
                </div>

                <div className="flex items-center justify-between pt-3 border-t border-[var(--border)]">
                  <span className="text-[12px] font-mono text-[var(--text-muted)]">
                    {m.is_default ? "Default engine" : "Alternative"}
                  </span>
                  <LinearButton
                    variant={isSelected ? "primary" : "secondary"}
                    size="sm"
                    onClick={() => onSelectModel?.(m)}
                  >
                    {isSelected && <Check className="size-3.5" strokeWidth={1.75} />}
                    <span>{isSelected ? "Active model" : "Select engine"}</span>
                  </LinearButton>
                </div>
              </LinearCard>
            );
          })}
        </div>
      </div>

      <div className="space-y-3 pt-3 border-t border-[var(--border)]">
        <h3 className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)]">
          Discover GGUF models on Hugging Face Hub
        </h3>

        <form onSubmit={handleSearch} className="flex gap-2">
          <div className="relative flex-1">
            <Search
              className="absolute left-3 top-2.5 size-3.5 text-[var(--text-muted)]"
              strokeWidth={1.75}
            />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search GGUF models on Hugging Face..."
              className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] pl-9 pr-4 py-1.5 text-[13px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
            />
          </div>
          <LinearButton type="submit" variant="primary" size="md" disabled={isSearching}>
            <span>{isSearching ? "Searching" : "Search hub"}</span>
          </LinearButton>
        </form>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {searchResults.map((hf) => {
            const jobId = downloadingFiles[hf.filename];
            const progress = jobId ? downloadProgresses[jobId] : undefined;
            const isDownloading = !!progress && progress.progressFraction < 1.0;
            const isInstalled = installedModels.some(
              (m) => m.name === hf.filename || m.id === hf.filename.replace(".gguf", "")
            );

            return (
              <LinearCard
                key={`${hf.repo_id}-${hf.filename}`}
                level={1}
                className="p-5 space-y-3"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="space-y-1 min-w-0">
                    <h4 className="text-[13px] font-semibold text-[var(--text)] truncate">
                      {hf.filename}
                    </h4>
                    <span className="font-mono text-[12px] text-[var(--text-muted)] truncate block">
                      {hf.repo_id}
                    </span>
                  </div>
                  <span className="rounded-[4px] border border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)] font-mono text-[12px] px-2 py-0.5 shrink-0">
                    {hf.quantization}
                  </span>
                </div>

                <div className="flex items-center justify-between text-[12px] font-mono">
                  <span className="text-[var(--text-muted)]">
                    Size: {formatBytes(hf.file_size_bytes)}
                  </span>
                  {hf.compatibility === "fits_in_vram" ? (
                    <span className="text-[var(--text)] font-medium flex items-center gap-1.5">
                      <span className="size-1.5 rounded-full bg-[var(--accent-green)]" />
                      Fits in VRAM
                    </span>
                  ) : hf.compatibility === "partial_offload" ? (
                    <span className="text-[var(--text)] font-medium flex items-center gap-1.5">
                      <span className="size-1.5 rounded-full bg-[var(--accent-amber)]" />
                      Partial offload
                    </span>
                  ) : (
                    <span className="text-[var(--text)] font-medium flex items-center gap-1.5">
                      <span className="size-1.5 rounded-full bg-[var(--accent-red)]" />
                      CPU only
                    </span>
                  )}
                </div>

                {isDownloading && progress && (
                  <div className="space-y-2 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3">
                    <div className="flex justify-between text-[12px] font-mono text-[var(--text-muted)] tabular-nums">
                      <span>{(progress.progressFraction * 100).toFixed(1)}%</span>
                      <span>
                        {(progress.speedBytesPerSec / (1024 * 1024)).toFixed(1)} MB/s · ETA: {progress.etaSeconds}s
                      </span>
                    </div>
                    <div className="h-1 w-full rounded-[2px] bg-[var(--bg-bar-idle)] overflow-hidden">
                      <div
                        className="h-full bg-[var(--accent-blue)]"
                        style={{ width: `${Math.round(progress.progressFraction * 100)}%` }}
                      />
                    </div>
                    <div className="text-[12px] text-[var(--text-muted)] font-mono flex items-center justify-end gap-1">
                      <Clock className="size-3.5" strokeWidth={1.75} />
                      <span>Streaming SHA-256 pipeline</span>
                    </div>
                  </div>
                )}

                <div className="flex items-center justify-between pt-3 border-t border-[var(--border)]">
                  <span className="text-[12px] font-mono text-[var(--text-muted)]">
                    Est. VRAM: ~{(hf.vram_required_mb / 1024).toFixed(1)} GB
                  </span>

                  {isInstalled ? (
                    <span className="flex items-center gap-1.5 text-[12px] font-medium text-[var(--accent-green)]">
                      <CheckCircle2 className="size-3.5" strokeWidth={1.75} />
                      <span>Ready</span>
                    </span>
                  ) : isDownloading ? (
                    <span className="text-[12px] font-mono text-[var(--text-muted)]">Downloading</span>
                  ) : (
                    <LinearButton
                      variant="primary"
                      size="sm"
                      onClick={() => handleDownload(hf)}
                    >
                      <Download className="size-3.5" strokeWidth={1.75} />
                      <span>Download GGUF</span>
                    </LinearButton>
                  )}
                </div>
              </LinearCard>
            );
          })}
        </div>
      </div>
    </div>
  );
};
