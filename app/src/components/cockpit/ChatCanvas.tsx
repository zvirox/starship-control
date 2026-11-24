import React, { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Workflow,
  User,
  ChevronDown,
  ChevronRight,
  Clock,
  AlertCircle,
  Brain,
  AppWindow,
  Film,
  Activity,
  HardDrive,
} from "lucide-react";
import { SanitizedMarkdown } from "../common/SanitizedMarkdown";
import type {
  ThoughtStreamEvent,
  TokenStreamEvent,
  StreamFinishedEvent,
  StreamErrorEvent,
  MissionMessageRecord,
} from "../../types/ipc";

export interface ChatCanvasProps {
  missionId: number;
  isStreaming: boolean;
  onStreamFinished?: () => void;
  onSelectSuggestion?: (prompt: string) => void;
  onNavigateTab?: (tab: "workflows" | "apps" | "models") => void;
}

interface DisplayMessage {
  id: string | number;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  thought?: string | null;
  timestamp?: string;
  isError?: boolean;
}

export const ChatCanvas: React.FC<ChatCanvasProps> = ({
  missionId,
  isStreaming,
  onStreamFinished,
  onSelectSuggestion,
  onNavigateTab,
}) => {
  const [messages, setMessages] = useState<DisplayMessage[]>([]);
  const [liveContent, setLiveContent] = useState<string>("");
  const [liveThought, setLiveThought] = useState<string>("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [expandedThoughts, setExpandedThoughts] = useState<Record<string | number, boolean>>({});
  const [showLiveThought, setShowLiveThought] = useState<boolean>(true);
  const [elapsedSeconds, setElapsedSeconds] = useState<number>(0);
  const scrollEndRef = useRef<HTMLDivElement>(null);

  // Load message history from SQLite for the active mission
  const loadMissionMessages = useCallback(async (mId: number) => {
    try {
      const records = await invoke<MissionMessageRecord[]>("cmd_mission_list_messages", {
        missionId: mId,
      });
      setMessages(
        records.map((r) => ({
          id: r.id,
          role: r.role,
          content: r.content,
          thought: r.thought_process,
          timestamp: r.created_at,
        }))
      );
      setErrorMessage(null);
    } catch (err) {
      console.warn("Failed to load mission messages:", err);
    }
  }, []);

  useEffect(() => {
    if (missionId) {
      loadMissionMessages(missionId);
    }
  }, [missionId, loadMissionMessages]);

  // Streaming elapsed timer
  useEffect(() => {
    let timer: NodeJS.Timeout | null = null;
    if (isStreaming) {
      setElapsedSeconds(0);
      setErrorMessage(null);
      timer = setInterval(() => {
        setElapsedSeconds((prev) => prev + 1);
      }, 1000);
    } else {
      setElapsedSeconds(0);
    }
    return () => {
      if (timer) clearInterval(timer);
    };
  }, [isStreaming]);

  // Real-time Tauri IPC event listeners
  useEffect(() => {
    let unlistenToken: (() => void) | undefined;
    let unlistenThought: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    async function setupListeners() {
      unlistenToken = await listen<TokenStreamEvent>("evt://token/stream", (event) => {
        if (event.payload.missionId === missionId) {
          setLiveContent((prev) => prev + event.payload.chunk);
        }
      });

      unlistenThought = await listen<ThoughtStreamEvent>("evt://thought/stream", (event) => {
        if (event.payload.missionId === missionId) {
          setLiveThought((prev) => prev + event.payload.thoughtChunk);
        }
      });

      unlistenFinished = await listen<StreamFinishedEvent>("evt://stream/finished", (event) => {
        if (event.payload.missionId === missionId) {
          setMessages((prev) => [
            ...prev,
            {
              id: event.payload.messageId || Date.now(),
              role: "assistant",
              content: event.payload.content,
              thought: event.payload.thought,
            },
          ]);
          setLiveContent("");
          setLiveThought("");
          setErrorMessage(null);
          onStreamFinished?.();
        }
      });

      unlistenError = await listen<StreamErrorEvent>("evt://stream/error", (event) => {
        if (event.payload.missionId === missionId) {
          setErrorMessage(event.payload.message || "Model inference failed. Please check your model settings.");
          setLiveContent("");
          setLiveThought("");
          onStreamFinished?.();
        }
      });
    }

    setupListeners().catch(console.warn);

    return () => {
      if (unlistenToken) unlistenToken();
      if (unlistenThought) unlistenThought();
      if (unlistenFinished) unlistenFinished();
      if (unlistenError) unlistenError();
    };
  }, [missionId, onStreamFinished]);

  useEffect(() => {
    scrollEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, liveContent, liveThought, errorMessage]);

  const toggleThought = (id: string | number) => {
    setExpandedThoughts((prev) => ({
      ...prev,
      [id]: !prev[id],
    }));
  };

  const suggestions = [
    {
      icon: AppWindow,
      title: "Connected apps",
      prompt: "What desktop applications are connected to Starship Control?",
    },
    {
      icon: Film,
      title: "Screencast workflow",
      prompt: "Record screen with SSP and clean audio with ALP",
    },
    {
      icon: HardDrive,
      title: "System maintenance",
      prompt: "Inspect system health and optimize build caches",
    },
    {
      icon: Activity,
      title: "Network status",
      prompt: "Check active network sockets and telemetry with Net Watch Pro",
    },
  ];

  return (
    <div className="min-w-0 flex-1 space-y-4 overflow-y-auto px-4 py-3 font-sans">
      {messages.length === 0 && !isStreaming && (
        <div className="mx-auto flex max-w-xl flex-col items-center justify-center py-16 text-center">
          <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl border border-border-subtle bg-white/[0.03] text-text-secondary">
            <Workflow className="h-6 w-6" strokeWidth={1.5} />
          </div>
          <h3 className="text-base font-medium text-text-primary">New mission</h3>
          <p className="mt-2 max-w-md text-xs leading-relaxed text-text-secondary">
            Coordinate ScreenStudio, Audio Labs, Easy Edits, Net Watch and Agentic Tuneup from one local command center
          </p>

          <div className="mt-6 grid w-full grid-cols-2 gap-2">
            {suggestions.map((item, idx) => {
              const Icon = item.icon;
              return (
                <button
                  key={idx}
                  onClick={() => onSelectSuggestion?.(item.prompt)}
                  className="flex items-start gap-3 rounded-xl border border-border-subtle bg-white/[0.03] p-3 text-left transition-colors duration-150 hover:bg-white/[0.05] focus-ring cursor-pointer"
                >
                  <Icon className="h-4 w-4 shrink-0 text-text-secondary" strokeWidth={1.5} />
                  <div className="min-w-0">
                    <span className="block text-xs font-semibold text-text-primary">{item.title}</span>
                    <span className="mt-1 block truncate text-[11px] text-text-secondary">{item.prompt}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}

      {messages.map((msg) => (
        <div
          key={msg.id}
          className={`flex gap-3 text-xs leading-relaxed ${
            msg.role === "user" ? "justify-end" : "justify-start"
          }`}
        >
          {msg.role !== "user" && (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-border-subtle bg-white/[0.05] text-text-secondary">
              <Workflow className="h-4 w-4" strokeWidth={1.5} />
            </div>
          )}

          <div className="max-w-2xl space-y-2">
            {msg.thought && (
              <div className="rounded-xl border border-border-subtle bg-white/[0.03] p-3">
                <button
                  onClick={() => toggleThought(msg.id)}
                  className="flex w-full items-center gap-2 text-left text-[11px] text-text-secondary transition-colors duration-150 hover:text-text-primary focus-ring cursor-pointer"
                >
                  <Brain className="h-3.5 w-3.5" strokeWidth={1.5} />
                  <span className="font-semibold text-text-primary">Reasoning process</span>
                  {expandedThoughts[msg.id] ? (
                    <ChevronDown className="ml-auto h-3.5 w-3.5 text-text-secondary" strokeWidth={1.5} />
                  ) : (
                    <ChevronRight className="ml-auto h-3.5 w-3.5 text-text-secondary" strokeWidth={1.5} />
                  )}
                </button>
                {expandedThoughts[msg.id] && (
                  <div className="mt-2 border-t border-border-subtle pt-2 font-mono text-[11px] leading-relaxed whitespace-pre-wrap text-text-primary/90">
                    {msg.thought}
                  </div>
                )}
              </div>
            )}

            <div
              className={`rounded-2xl p-3 text-xs ${
                msg.role === "user"
                  ? "ml-auto rounded-tr-sm border border-border-subtle bg-surface-solid text-text-primary"
                  : "rounded-tl-sm border border-border-subtle bg-raised text-text-primary"
              }`}
            >
              <SanitizedMarkdown content={msg.content} />
            </div>
          </div>

          {msg.role === "user" && (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-border-subtle bg-surface-solid text-text-secondary">
              <User className="h-4 w-4" strokeWidth={1.5} />
            </div>
          )}
        </div>
      ))}

      {isStreaming && (
        <div className="flex justify-start gap-3 text-xs leading-relaxed">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-border-subtle bg-white/[0.05] text-text-secondary">
            <Workflow className="h-4 w-4" strokeWidth={1.5} />
          </div>

          <div className="min-w-0 max-w-2xl flex-1 space-y-2">
            {liveThought && (
              <div className="rounded-xl border border-border-glass bg-white/[0.03] p-3">
                <button
                  type="button"
                  onClick={() => setShowLiveThought(!showLiveThought)}
                  className="flex w-full items-center justify-between text-left text-[11px] text-text-secondary focus-ring cursor-pointer"
                >
                  <div className="flex items-center gap-2">
                    <span className="h-1.5 w-1.5 rounded-full bg-signal-thinking" />
                    <span className="font-medium text-text-primary">Thinking</span>
                  </div>
                  <div className="flex items-center gap-1 text-text-secondary font-medium">
                    <Clock className="h-3.5 w-3.5" strokeWidth={1.5} />
                    <span className="tabular">{elapsedSeconds}s</span>
                  </div>
                </button>
                {showLiveThought && (
                  <pre className="mt-2 max-h-36 overflow-y-auto whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-text-primary/90">
                    {liveThought}
                  </pre>
                )}
              </div>
            )}

            <div className="rounded-2xl rounded-tl-sm border border-border-subtle bg-raised p-3 text-text-primary">
              {liveContent ? (
                <div className="font-sans">
                  <SanitizedMarkdown content={liveContent} />
                  <span className="ml-1 inline-block h-3.5 w-1.5 animate-pulse bg-accent align-middle" />
                </div>
              ) : (
                <div className="flex items-center gap-2 text-xs text-text-secondary">
                  <span className="h-1.5 w-1.5 rounded-full bg-signal-thinking" />
                  <span>Generating response</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {errorMessage && (
        <div className="flex max-w-2xl items-start gap-3 rounded-xl border border-signal-gate/30 bg-signal-gate-bg p-3 text-xs text-signal-gate">
          <AlertCircle className="h-4 w-4 shrink-0" strokeWidth={1.5} />
          <div className="flex-1">
            <span className="block font-medium">Inference error</span>
            <span className="mt-1 block text-[11px] text-text-secondary">{errorMessage}</span>
          </div>
          {onNavigateTab && (
            <button
              onClick={() => onNavigateTab("models")}
              className="text-[11px] text-text-secondary underline hover:text-text-primary focus-ring cursor-pointer"
            >
              Open models
            </button>
          )}
        </div>
      )}

      <div ref={scrollEndRef} />
    </div>
  );
};
