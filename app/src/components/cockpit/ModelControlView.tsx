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
import { GlassPanel } from "../common/GlassPanel";
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
    <div className="flex-1 space-y-5 overflow-y-auto p-5 font-sans">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h2 className="text-base font-medium text-text-primary flex items-center gap-2">
            <Cpu className="h-4 w-4 text-accent" strokeWidth={1.5} />
            <span>Model control and Hugging Face Hub</span>
          </h2>
          <p className="text-xs text-text-secondary">
            Discover, download, and verify local GGUF quantizations directly on your hardware
          </p>
        </div>

        {statusMessage && (
          <div className="flex items-center gap-1.5 rounded-lg border border-border-subtle bg-white/[0.03] px-3 py-2 text-xs text-text-secondary">
            <AlertCircle className="h-3.5 w-3.5 text-accent" strokeWidth={1.5} />
            <span>{statusMessage}</span>
          </div>
        )}
      </div>

      {/* Installed Models Section */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-secondary flex items-center gap-2">
          <HardDrive className="h-3.5 w-3.5 text-text-secondary" strokeWidth={1.5} />
          <span>Installed and registered models ({installedModels.length})</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {installedModels.map((m) => {
            const isSelected = m.id === activeModelId;
            return (
              <GlassPanel
                key={m.id}
                recipe="pane"
                className="p-4 space-y-3 hairline transition-colors"
              >
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <h4
                      className={`text-xs font-medium ${
                        isSelected ? "text-accent" : "text-text-primary"
                      }`}
                    >
                      {m.name}
                    </h4>
                    <span
                      className={`font-mono text-[11px] block font-medium ${
                        m.provider_type === "local_gguf"
                          ? "text-signal-local-model"
                          : "text-text-secondary"
                      }`}
                    >
                      {m.provider_type === "local_gguf" ? "Local GGUF · Vulkan" : "Cloud remote"}
                    </span>
                  </div>
                  <span className="inline-flex items-center gap-1.5 rounded-lg border border-border-subtle bg-white/[0.03] px-2 py-1 font-mono text-[11px] text-text-secondary">
                    <span
                      className={`h-1.5 w-1.5 rounded-full ${
                        m.download_status === "ready" ? "bg-signal-success" : "bg-signal-thinking"
                      }`}
                    />
                    {m.download_status}
                  </span>
                </div>

                <div className="text-[11px] font-mono text-text-secondary space-y-1">
                  <div>Context window: {m.context_window} tokens</div>
                  {m.file_size_bytes && <div>Size: {formatBytes(m.file_size_bytes)}</div>}
                  {m.file_path && (
                    <div className="truncate text-text-secondary font-mono">Path: {m.file_path}</div>
                  )}
                </div>

                <div className="flex items-center justify-between pt-3 border-t border-border-subtle">
                  <span className="text-[11px] text-text-secondary font-mono font-medium">
                    {m.is_default ? "Default engine" : "Alternative"}
                  </span>
                  <button
                    onClick={() => onSelectModel?.(m)}
                    className={`flex items-center gap-1 rounded-lg px-2 py-1.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:text-accent ${
                      isSelected
                        ? "bg-accent-muted text-accent border border-accent/30"
                        : "bg-white/[0.03] hover:bg-white/10 text-text-secondary"
                    }`}
                  >
                    {isSelected && <Check className="h-3.5 w-3.5 text-accent" strokeWidth={1.5} />}
                    <span>{isSelected ? "Active model" : "Select engine"}</span>
                  </button>
                </div>
              </GlassPanel>
            );
          })}
        </div>
      </div>

      {/* Hugging Face Hub Search & Download Section */}
      <div className="space-y-3 pt-2 border-t border-border-subtle">
        <h3 className="text-xs font-medium text-text-secondary">
          Discover GGUF models on Hugging Face Hub
        </h3>

        {/* Search Bar */}
        <form onSubmit={handleSearch} className="flex gap-2">
          <div className="relative flex-1">
            <Search
              className="absolute left-3 top-3 h-4 w-4 text-text-secondary"
              strokeWidth={1.5}
            />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search GGUF models on Hugging Face..."
              className="w-full rounded-xl border border-border-subtle bg-surface-solid pl-9 pr-4 py-2 text-xs font-medium text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent font-sans"
            />
          </div>
          <button
            type="submit"
            disabled={isSearching}
            className="flex items-center gap-2 rounded-xl bg-accent hover:bg-accent-hover px-4 py-2 text-xs font-semibold text-accent-text transition-colors disabled:opacity-40 focus-visible:outline-none cursor-pointer"
          >
            <span>{isSearching ? "Searching" : "Search hub"}</span>
          </button>
        </form>

        {/* Search Results Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {searchResults.map((hf) => {
            const jobId = downloadingFiles[hf.filename];
            const progress = jobId ? downloadProgresses[jobId] : undefined;
            const isDownloading = !!progress && progress.progressFraction < 1.0;
            const isInstalled = installedModels.some(
              (m) => m.name === hf.filename || m.id === hf.filename.replace(".gguf", "")
            );

            return (
              <GlassPanel
                key={`${hf.repo_id}-${hf.filename}`}
                recipe="pane"
                className="p-4 space-y-3 hairline"
              >
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <h4 className="text-xs font-semibold text-text-primary line-clamp-1">
                      {hf.filename}
                    </h4>
                    <span className="font-mono text-[11px] text-text-secondary font-medium truncate block max-w-[200px]">
                      {hf.repo_id}
                    </span>
                  </div>
                  <span className="rounded-lg border border-border-subtle bg-white/[0.03] text-text-secondary font-mono text-[11px] px-2 py-1">
                    {hf.quantization}
                  </span>
                </div>

                {/* Compatibility Badges */}
                <div className="flex items-center justify-between text-[11px] font-mono">
                  <span className="text-text-secondary font-medium">Size: {formatBytes(hf.file_size_bytes)}</span>
                  {hf.compatibility === "fits_in_vram" ? (
                    <span className="text-text-primary font-medium flex items-center gap-1.5">
                      <span className="h-1.5 w-1.5 rounded-full bg-signal-success" />
                      Fits in VRAM
                    </span>
                  ) : hf.compatibility === "partial_offload" ? (
                    <span className="text-text-primary font-medium flex items-center gap-1.5">
                      <span className="h-1.5 w-1.5 rounded-full bg-signal-thinking" />
                      Partial offload
                    </span>
                  ) : (
                    <span className="text-text-primary font-medium flex items-center gap-1.5">
                      <span className="h-1.5 w-1.5 rounded-full bg-signal-gate" />
                      CPU only
                    </span>
                  )}
                </div>

                {/* Download Progress Bar if downloading */}
                {isDownloading && progress && (
                  <div className="space-y-2 rounded-lg border border-border-subtle bg-surface-solid p-3">
                    <div className="flex justify-between text-[11px] font-mono text-text-secondary tabular font-medium">
                      <span>{(progress.progressFraction * 100).toFixed(1)}%</span>
                      <span>
                        {(progress.speedBytesPerSec / (1024 * 1024)).toFixed(1)} MB/s · ETA: {progress.etaSeconds}s
                      </span>
                    </div>
                    <div className="h-1.5 w-full rounded-lg bg-border-subtle overflow-hidden">
                      <div
                        className="h-full bg-accent"
                        style={{ width: `${Math.round(progress.progressFraction * 100)}%` }}
                      />
                    </div>
                    <div className="text-[11px] text-text-secondary font-mono flex items-center justify-end gap-1">
                      <Clock className="h-3.5 w-3.5" strokeWidth={1.5} />
                      <span>Streaming SHA-256 pipeline</span>
                    </div>
                  </div>
                )}

                {/* Action Buttons */}
                <div className="flex items-center justify-between pt-3 border-t border-border-subtle">
                  <span className="text-[11px] font-mono text-text-secondary font-medium">
                    Est. VRAM: ~{(hf.vram_required_mb / 1024).toFixed(1)} GB
                  </span>

                  {isInstalled ? (
                    <span className="flex items-center gap-1.5 text-[11px] font-medium text-signal-success">
                      <CheckCircle2 className="h-3.5 w-3.5" strokeWidth={1.5} />
                      <span>Ready</span>
                    </span>
                  ) : isDownloading ? (
                    <span className="text-[11px] font-mono text-text-secondary">Downloading</span>
                  ) : (
                    <button
                      onClick={() => handleDownload(hf)}
                      className="flex items-center gap-1.5 rounded-lg bg-accent hover:bg-accent-hover px-3 py-1.5 text-xs font-medium text-accent-text transition-colors focus-visible:outline-none"
                    >
                      <Download className="h-3.5 w-3.5" strokeWidth={1.5} />
                      <span>Download GGUF</span>
                    </button>
                  )}
                </div>
              </GlassPanel>
            );
          })}
        </div>
      </div>
    </div>
  );
};
