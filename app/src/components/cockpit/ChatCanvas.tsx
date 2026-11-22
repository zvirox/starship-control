import React, { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Sparkles,
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
    <div className="flex-1 space-y-4 overflow-y-auto px-4 py-3 font-sans">
      {/* Empty State / Welcome */}
      {messages.length === 0 && !isStreaming && (
        <div className="flex flex-col items-center justify-center py-16 text-center max-w-xl mx-auto">
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-white/[0.04] border border-white/[0.08] text-sky-400 mb-4 shadow-sm">
            <Sparkles className="h-6 w-6" />
          </div>
          <h3 className="text-base font-semibold text-slate-100">How can Starship Control help you?</h3>
          <p className="text-xs text-slate-400 max-w-md mt-1.5 leading-relaxed">
            Your local AI command center. Coordinate ScreenStudio, Audio Labs, Easy Edits, Net Watch, and Agentic Tuneup directly from this chat.
          </p>

          {/* Quick Suggestions */}
          <div className="grid grid-cols-2 gap-2.5 w-full mt-6">
            {suggestions.map((item, idx) => {
              const Icon = item.icon;
              return (
                <button
                  key={idx}
                  onClick={() => onSelectSuggestion?.(item.prompt)}
                  className="flex items-start gap-2.5 p-3 text-left rounded-xl border border-white/[0.06] bg-white/[0.02] hover:bg-white/[0.05] hover:border-white/[0.12] transition-colors group cursor-pointer"
                >
                  <Icon className="h-4 w-4 text-slate-400 group-hover:text-sky-400 transition-colors shrink-0 mt-0.5" />
                  <div>
                    <span className="text-xs font-medium text-slate-200 block">{item.title}</span>
                    <span className="text-[11px] text-slate-500 line-clamp-1 mt-0.5">{item.prompt}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}

      {/* Render Conversation Messages */}
      {messages.map((msg) => (
        <div
          key={msg.id}
          className={`flex gap-3 text-xs leading-relaxed ${
            msg.role === "user" ? "justify-end" : "justify-start"
          }`}
        >
          {msg.role !== "user" && (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl bg-white/[0.05] text-slate-300 border border-white/[0.08]">
              <Sparkles className="h-3.5 w-3.5 text-sky-400" />
            </div>
          )}

          <div className="max-w-2xl space-y-2">
            {/* Thought Accordion if present */}
            {msg.thought && (
              <div className="rounded-xl border border-white/[0.06] bg-white/[0.02] p-2.5">
                <button
                  onClick={() => toggleThought(msg.id)}
                  className="flex items-center gap-1.5 text-[11px] text-slate-400 hover:text-slate-200 transition-colors w-full text-left cursor-pointer"
                >
                  <Brain className="h-3 w-3 text-slate-400" />
                  <span className="font-medium">Reasoning process</span>
                  {expandedThoughts[msg.id] ? (
                    <ChevronDown className="h-3 w-3 ml-auto text-slate-500" />
                  ) : (
                    <ChevronRight className="h-3 w-3 ml-auto text-slate-500" />
                  )}
                </button>
                {expandedThoughts[msg.id] && (
                  <div className="mt-2 text-[11px] text-slate-300 border-t border-white/[0.06] pt-2 leading-relaxed whitespace-pre-wrap font-mono">
                    {msg.thought}
                  </div>
                )}
              </div>
            )}

            {/* Message Body */}
            <div
              className={`rounded-2xl p-3.5 text-xs ${
                msg.role === "user"
                  ? "bg-[#1E232E] text-slate-100 border border-white/[0.08] rounded-tr-sm ml-auto"
                  : "bg-[#12151B] text-slate-200 border border-white/[0.06] rounded-tl-sm shadow-sm"
              }`}
            >
              <div className="whitespace-pre-wrap leading-relaxed">{msg.content}</div>
            </div>
          </div>

          {msg.role === "user" && (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl bg-slate-800 text-slate-300 border border-white/[0.08]">
              <User className="h-3.5 w-3.5" />
            </div>
          )}
        </div>
      ))}

      {/* Live Active Streaming Bubble */}
      {isStreaming && (
        <div className="flex gap-3 text-xs leading-relaxed justify-start">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl bg-white/[0.05] text-sky-400 border border-white/[0.08]">
            <Sparkles className="h-3.5 w-3.5 animate-pulse" />
          </div>

          <div className="max-w-2xl space-y-2 flex-1">
            {/* Live Thought Trace */}
            {liveThought && (
              <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-2.5">
                <button
                  type="button"
                  onClick={() => setShowLiveThought(!showLiveThought)}
                  className="flex items-center justify-between text-[11px] text-slate-300 w-full text-left cursor-pointer"
                >
                  <div className="flex items-center gap-1.5">
                    <span className="h-1.5 w-1.5 rounded-full bg-sky-400 animate-pulse" />
                    <span className="font-medium text-slate-300">Thinking...</span>
                  </div>
                  <div className="flex items-center gap-1 text-slate-500 text-[10px]">
                    <Clock className="h-3 w-3" />
                    <span>{elapsedSeconds}s</span>
                  </div>
                </button>
                {showLiveThought && (
                  <pre className="mt-2 whitespace-pre-wrap font-mono text-[11px] text-slate-400 leading-relaxed max-h-36 overflow-y-auto">
                    {liveThought}
                  </pre>
                )}
              </div>
            )}

            {/* Live Content Stream */}
            <div className="rounded-2xl rounded-tl-sm bg-[#12151B] p-3.5 text-slate-200 border border-white/[0.06] shadow-sm">
              {liveContent ? (
                <div className="whitespace-pre-wrap leading-relaxed font-sans">
                  {liveContent}
                  <span className="inline-block w-1.5 h-3.5 bg-sky-400 ml-1 animate-pulse align-middle" />
                </div>
              ) : (
                <div className="flex items-center gap-2 text-slate-400 text-xs">
                  <span className="h-2 w-2 rounded-full bg-sky-400 animate-ping" />
                  <span>Generating response...</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Stream Error Notice */}
      {errorMessage && (
        <div className="flex items-start gap-2.5 p-3 rounded-xl border border-rose-500/20 bg-rose-500/5 text-rose-300 text-xs max-w-2xl">
          <AlertCircle className="h-4 w-4 shrink-0 text-rose-400 mt-0.5" />
          <div className="flex-1">
            <span className="font-medium block">Inference Error</span>
            <span className="text-[11px] text-rose-400/80 block mt-0.5">{errorMessage}</span>
          </div>
          {onNavigateTab && (
            <button
              onClick={() => onNavigateTab("models")}
              className="text-[11px] underline hover:text-rose-200 cursor-pointer"
            >
              Open Models
            </button>
          )}
        </div>
      )}

      <div ref={scrollEndRef} />
    </div>
  );
};
