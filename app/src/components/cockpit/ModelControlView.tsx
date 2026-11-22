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
  Sparkles,
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
        <div>
          <h2 className="text-base font-bold text-slate-100 flex items-center gap-2">
            <Cpu className="h-5 w-5 text-cyan-400" />
            <span>Model Control & Hugging Face Hub</span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Discover, download, and verify local GGUF quantizations directly on your hardware.
          </p>
        </div>

        {statusMessage && (
          <div className="flex items-center gap-1.5 rounded-lg border border-cyan-400/40 bg-cyan-500/10 px-3 py-1.5 text-xs text-cyan-300">
            <AlertCircle className="h-3.5 w-3.5" />
            <span>{statusMessage}</span>
          </div>
        )}
      </div>

      {/* Installed Models Section */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold tracking-wider uppercase text-slate-400 flex items-center gap-2">
          <HardDrive className="h-3.5 w-3.5 text-purple-400" />
          <span>Installed & Registered Models ({installedModels.length})</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {installedModels.map((m) => {
            const isSelected = m.id === activeModelId;
            return (
              <GlassPanel
                key={m.id}
                recipe="pane"
                className={`p-3.5 space-y-2 border transition-all ${
                  isSelected ? "border-purple-400 bg-purple-500/10 shadow-[0_0_16px_rgba(168,85,247,0.2)]" : "border-white/10"
                }`}
              >
                <div className="flex items-start justify-between">
                  <div>
                    <h4 className="font-semibold text-xs text-slate-100">{m.name}</h4>
                    <span className="font-mono text-[10px] text-purple-300 block">
                      {m.provider_type === "local_gguf" ? "Local GGUF · Vulkan" : "Cloud Remote"}
                    </span>
                  </div>
                  <span
                    className={`rounded px-1.5 py-0.5 text-[9px] font-mono border ${
                      m.download_status === "ready"
                        ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/30"
                        : "bg-cyan-500/20 text-cyan-300 border-cyan-500/30"
                    }`}
                  >
                    {m.download_status.toUpperCase()}
                  </span>
                </div>

                <div className="text-[10px] font-mono text-slate-400 space-y-0.5">
                  <div>Context Window: {m.context_window} tokens</div>
                  {m.file_size_bytes && <div>Size: {formatBytes(m.file_size_bytes)}</div>}
                  {m.file_path && <div className="truncate text-slate-500">Path: {m.file_path}</div>}
                </div>

                <div className="flex items-center justify-between pt-1 border-t border-white/5">
                  <span className="text-[10px] text-slate-500 font-mono">
                    {m.is_default ? "Default Engine" : "Alternative"}
                  </span>
                  <button
                    onClick={() => onSelectModel?.(m)}
                    className={`flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors ${
                      isSelected
                        ? "bg-purple-500/30 text-purple-200 border border-purple-400/40"
                        : "bg-white/5 hover:bg-white/10 text-slate-300"
                    }`}
                  >
                    {isSelected && <Check className="h-3 w-3 text-purple-300" />}
                    <span>{isSelected ? "Active Model" : "Select Engine"}</span>
                  </button>
                </div>
              </GlassPanel>
            );
          })}
        </div>
      </div>

      {/* Hugging Face Hub Search & Download Section */}
      <div className="space-y-3 pt-2 border-t border-white/10">
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-semibold tracking-wider uppercase text-slate-400 flex items-center gap-2">
            <Sparkles className="h-3.5 w-3.5 text-cyan-400" />
            <span>Discover GGUF Models on Hugging Face Hub</span>
          </h3>
        </div>

        {/* Search Bar */}
        <form onSubmit={handleSearch} className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search Hugging Face (e.g. 'qwen2.5-coder', 'llama-3.2', 'deepseek')..."
              className="w-full rounded-xl border border-white/10 bg-black/40 pl-9 pr-4 py-2 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400 font-sans"
            />
          </div>
          <button
            type="submit"
            disabled={isSearching}
            className="flex items-center gap-2 rounded-xl bg-cyan-400 hover:bg-cyan-300 px-4 py-2 text-xs font-semibold text-slate-950 shadow-[0_0_16px_rgba(56,189,248,0.3)] transition-transform active:scale-95 disabled:opacity-50"
          >
            <span>{isSearching ? "Searching..." : "Search Hub"}</span>
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
              <GlassPanel key={`${hf.repo_id}-${hf.filename}`} recipe="pane" className="p-3.5 space-y-2.5 border border-white/10">
                <div className="flex items-start justify-between">
                  <div>
                    <h4 className="font-semibold text-xs text-slate-100 line-clamp-1">{hf.filename}</h4>
                    <span className="font-mono text-[10px] text-slate-400 truncate block max-w-[200px]">
                      {hf.repo_id}
                    </span>
                  </div>
                  <span className="rounded bg-white/10 text-slate-300 font-mono text-[9px] px-1.5 py-0.5 border border-white/15">
                    {hf.quantization}
                  </span>
                </div>

                {/* Compatibility Badges */}
                <div className="flex items-center justify-between text-[10px] font-mono">
                  <span className="text-slate-400">Size: {formatBytes(hf.file_size_bytes)}</span>
                  {hf.compatibility === "fits_in_vram" ? (
                    <span className="text-emerald-400 flex items-center gap-1 font-semibold">
                      <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                      Fits in VRAM (Fastest)
                    </span>
                  ) : hf.compatibility === "partial_offload" ? (
                    <span className="text-amber-400 flex items-center gap-1 font-semibold">
                      <span className="h-1.5 w-1.5 rounded-full bg-amber-400" />
                      Partial Offload
                    </span>
                  ) : (
                    <span className="text-rose-400 flex items-center gap-1 font-semibold">
                      <span className="h-1.5 w-1.5 rounded-full bg-rose-400" />
                      CPU Only (Slow)
                    </span>
                  )}
                </div>

                {/* Download Progress Bar if downloading */}
                {isDownloading && progress && (
                  <div className="space-y-1.5 rounded-lg bg-black/40 p-2 border border-cyan-500/20">
                    <div className="flex justify-between text-[10px] font-mono text-cyan-300">
                      <span>{(progress.progressFraction * 100).toFixed(1)}%</span>
                      <span>
                        {(progress.speedBytesPerSec / (1024 * 1024)).toFixed(1)} MB/s · ETA: {progress.etaSeconds}s
                      </span>
                    </div>
                    <div className="h-1.5 w-full rounded-full bg-white/10 overflow-hidden">
                      <div
                        className="h-full bg-cyan-400 transition-all duration-300 shadow-[0_0_8px_rgba(56,189,248,0.8)]"
                        style={{ width: `${Math.round(progress.progressFraction * 100)}%` }}
                      />
                    </div>
                    <div className="text-[9px] text-slate-400 font-mono text-right flex items-center justify-end gap-1">
                      <Clock className="h-2.5 w-2.5 text-cyan-400" />
                      <span>Streaming SHA-256 Pipeline...</span>
                    </div>
                  </div>
                )}

                {/* Action Buttons */}
                <div className="flex items-center justify-between pt-1 border-t border-white/5">
                  <span className="text-[10px] font-mono text-slate-500">
                    Est. VRAM: ~{(hf.vram_required_mb / 1024).toFixed(1)} GB
                  </span>

                  {isInstalled ? (
                    <span className="flex items-center gap-1 text-[11px] font-medium text-emerald-400">
                      <CheckCircle2 className="h-3.5 w-3.5" />
                      <span>Ready</span>
                    </span>
                  ) : isDownloading ? (
                    <span className="text-[11px] font-mono text-cyan-300 animate-pulse">Downloading...</span>
                  ) : (
                    <button
                      onClick={() => handleDownload(hf)}
                      className="flex items-center gap-1 rounded-lg bg-cyan-400 hover:bg-cyan-300 px-3 py-1 text-xs font-semibold text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.3)] transition-transform active:scale-95"
                    >
                      <Download className="h-3 w-3" />
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
