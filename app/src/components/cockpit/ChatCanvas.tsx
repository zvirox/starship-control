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
          <div className="mb-4 flex size-12 items-center justify-center rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text-muted)]">
            <Workflow className="size-6" strokeWidth={1.75} />
          </div>
          <h3 className="text-[16px] font-semibold text-[var(--text)]">New mission</h3>
          <p className="mt-2 max-w-md text-[12px] leading-relaxed text-[var(--text-muted)]">
            Coordinate ScreenStudio, Audio Labs, Easy Edits, Net Watch and Agentic Tuneup from one local command center
          </p>

          <div className="mt-6 grid w-full grid-cols-2 gap-2">
            {suggestions.map((item, idx) => {
              const Icon = item.icon;
              return (
                <button
                  key={idx}
                  type="button"
                  onClick={() => onSelectSuggestion?.(item.prompt)}
                  className="flex items-start gap-3 rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-left transition-colors duration-150 hover:bg-[var(--bg-surface-2)] hover:border-[var(--border-strong)] focus-ring cursor-pointer"
                >
                  <Icon className="size-4 shrink-0 text-[var(--accent-blue)]" strokeWidth={1.75} />
                  <div className="min-w-0">
                    <span className="block text-[12px] font-semibold text-[var(--text)]">{item.title}</span>
                    <span className="mt-1 block truncate text-[12px] text-[var(--text-muted)]">{item.prompt}</span>
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
          className={`flex gap-3 text-[13px] leading-relaxed ${
            msg.role === "user" ? "justify-end" : "justify-start"
          }`}
        >
          {msg.role !== "user" && (
            <div className="flex size-7 shrink-0 items-center justify-center rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text-muted)]">
              <Workflow className="size-4" strokeWidth={1.75} />
            </div>
          )}

          <div className="max-w-2xl space-y-2">
            {msg.thought && (
              <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-3">
                <button
                  type="button"
                  onClick={() => toggleThought(msg.id)}
                  className="flex w-full items-center gap-2 text-left text-[12px] text-[var(--text-muted)] transition-colors duration-150 hover:text-[var(--text)] focus-ring cursor-pointer"
                >
                  <Brain className="size-3.5" strokeWidth={1.75} />
                  <span className="font-semibold text-[var(--text)]">Reasoning process</span>
                  {expandedThoughts[msg.id] ? (
                    <ChevronDown className="ml-auto size-3.5 text-[var(--text-muted)]" strokeWidth={1.75} />
                  ) : (
                    <ChevronRight className="ml-auto size-3.5 text-[var(--text-muted)]" strokeWidth={1.75} />
                  )}
                </button>
                {expandedThoughts[msg.id] && (
                  <div className="mt-2 border-t border-[var(--border)] pt-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap text-[var(--text)]">
                    {msg.thought}
                  </div>
                )}
              </div>
            )}

            <div
              className={`rounded-[8px] p-3 text-[13px] border border-[var(--border)] text-[var(--text)] ${
                msg.role === "user"
                  ? "ml-auto bg-[var(--bg-surface-2)]"
                  : "bg-[var(--bg-surface)]"
              }`}
            >
              <SanitizedMarkdown content={msg.content} />
            </div>
          </div>

          {msg.role === "user" && (
            <div className="flex size-7 shrink-0 items-center justify-center rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)]">
              <User className="size-4" strokeWidth={1.75} />
            </div>
          )}
        </div>
      ))}

      {isStreaming && (
        <div className="flex justify-start gap-3 text-[13px] leading-relaxed">
          <div className="flex size-7 shrink-0 items-center justify-center rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text-muted)]">
            <Workflow className="size-4" strokeWidth={1.75} />
          </div>

          <div className="min-w-0 max-w-2xl flex-1 space-y-2">
            {liveThought && (
              <div className="rounded-[8px] border border-[var(--border-strong)] bg-[var(--bg-surface)] p-3">
                <button
                  type="button"
                  onClick={() => setShowLiveThought(!showLiveThought)}
                  className="flex w-full items-center justify-between text-left text-[12px] text-[var(--text-muted)] focus-ring cursor-pointer"
                >
                  <div className="flex items-center gap-2">
                    <span className="size-1.5 rounded-full bg-[var(--accent-amber)]" />
                    <span className="font-medium text-[var(--text)]">Thinking</span>
                  </div>
                  <div className="flex items-center gap-1 text-[var(--text-muted)] font-medium">
                    <Clock className="size-3.5" strokeWidth={1.75} />
                    <span className="tabular-nums">{elapsedSeconds}s</span>
                  </div>
                </button>
                {showLiveThought && (
                  <pre className="mt-2 max-h-36 overflow-y-auto whitespace-pre-wrap font-mono text-[12px] leading-relaxed text-[var(--text)]">
                    {liveThought}
                  </pre>
                )}
              </div>
            )}

            <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-[var(--text)]">
              {liveContent ? (
                <div className="font-sans">
                  <SanitizedMarkdown content={liveContent} />
                  <span className="ml-1 inline-block h-3.5 w-1.5 animate-pulse bg-[var(--accent-blue)] align-middle" />
                </div>
              ) : (
                <div className="flex items-center gap-2 text-[12px] text-[var(--text-muted)]">
                  <span className="size-1.5 rounded-full bg-[var(--accent-amber)]" />
                  <span>Generating response</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {errorMessage && (
        <div className="flex max-w-2xl items-start gap-3 rounded-[8px] border border-[var(--accent-red)] bg-[var(--tint-red)] p-3 text-[12px] text-[var(--accent-red)]">
          <AlertCircle className="size-4 shrink-0" strokeWidth={1.75} />
          <div className="flex-1">
            <span className="block font-medium">Inference error</span>
            <span className="mt-1 block text-[12px] text-[var(--text-muted)]">{errorMessage}</span>
          </div>
          {onNavigateTab && (
            <button
              type="button"
              onClick={() => onNavigateTab("models")}
              className="text-[12px] text-[var(--text-muted)] underline hover:text-[var(--text)] focus-ring cursor-pointer"
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
