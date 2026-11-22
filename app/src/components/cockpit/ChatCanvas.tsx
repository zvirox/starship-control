import React, { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Sparkles, User, ChevronDown, ChevronRight, Clock, Cpu } from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type { ThoughtStreamEvent, TokenStreamEvent, StreamFinishedEvent, MissionMessageRecord } from "../../types/ipc";

export interface ChatCanvasProps {
  missionId: number;
  initialMessages?: MissionMessageRecord[];
  isStreaming: boolean;
  onStreamFinished?: () => void;
}

interface DisplayMessage {
  id: string | number;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  thought?: string | null;
  timestamp?: string;
  latencyMs?: number;
}

export const ChatCanvas: React.FC<ChatCanvasProps> = ({
  missionId,
  initialMessages = [],
  isStreaming,
  onStreamFinished,
}) => {
  const [messages, setMessages] = useState<DisplayMessage[]>([]);
  const [liveContent, setLiveContent] = useState<string>("");
  const [liveThought, setLiveThought] = useState<string>("");
  const [expandedThoughts, setExpandedThoughts] = useState<Record<string | number, boolean>>({ live: true });
  const [elapsedSeconds, setElapsedSeconds] = useState<number>(0);
  const scrollEndRef = useRef<HTMLDivElement>(null);

  // Sync initial messages
  useEffect(() => {
    if (initialMessages.length > 0) {
      setMessages(
        initialMessages.map((m) => ({
          id: m.id,
          role: m.role,
          content: m.content,
          thought: m.thought_process,
          timestamp: m.created_at,
          latencyMs: m.latency_ms ?? undefined,
        }))
      );
    }
  }, [initialMessages]);

  // Latency timer during active streaming
  useEffect(() => {
    let timer: NodeJS.Timeout | null = null;
    if (isStreaming) {
      setElapsedSeconds(0);
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

  // Subscribe to real-time Tauri push events
  useEffect(() => {
    let unlistenToken: (() => void) | undefined;
    let unlistenThought: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;

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
          onStreamFinished?.();
        }
      });
    }

    setupListeners().catch(console.warn);

    return () => {
      if (unlistenToken) unlistenToken();
      if (unlistenThought) unlistenThought();
      if (unlistenFinished) unlistenFinished();
    };
  }, [missionId, onStreamFinished]);

  useEffect(() => {
    scrollEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, liveContent, liveThought]);

  const toggleThought = (id: string | number) => {
    setExpandedThoughts((prev) => ({
      ...prev,
      [id]: !prev[id],
    }));
  };

  return (
    <div className="flex-1 space-y-4 overflow-y-auto px-4 py-2 font-sans">
      {messages.length === 0 && !isStreaming && (
        <div className="flex flex-col items-center justify-center py-12 text-center text-slate-500">
          <Sparkles className="h-8 w-8 text-cyan-400/40 mb-3 animate-pulse" />
          <h4 className="text-sm font-semibold text-slate-300">Starship Control Session Ready</h4>
          <p className="text-xs text-slate-500 max-w-sm mt-1">
            Issue a natural language prompt or workflow instruction. Both local GGUF and cloud inference streams render here.
          </p>
        </div>
      )}

      {/* Render Past Messages */}
      {messages.map((msg) => (
        <div
          key={msg.id}
          className={`flex gap-3 text-xs leading-relaxed ${
            msg.role === "user" ? "justify-end" : "justify-start"
          }`}
        >
          {msg.role !== "user" && (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl bg-cyan-500/20 text-cyan-400 border border-cyan-400/30">
              <Sparkles className="h-3.5 w-3.5" />
            </div>
          )}

          <div className="max-w-2xl space-y-2">
            {/* Thought Accordion if present */}
            {msg.thought && (
              <GlassPanel recipe="pane" className="border-amber-500/20 bg-amber-500/5 p-2.5 rounded-xl">
                <button
                  onClick={() => toggleThought(msg.id)}
                  className="flex items-center gap-1.5 font-mono text-[11px] text-amber-300 hover:text-amber-200 transition-colors w-full text-left"
                >
                  {expandedThoughts[msg.id] ? (
                    <ChevronDown className="h-3 w-3 text-amber-400" />
                  ) : (
                    <ChevronRight className="h-3 w-3 text-amber-400" />
                  )}
                  <span className="font-semibold">Reasoning Process</span>
                  <span className="text-[10px] text-slate-500 ml-auto">Verified</span>
                </button>
                {expandedThoughts[msg.id] && (
                  <pre className="mt-2 whitespace-pre-wrap font-mono text-[11px] text-slate-300 border-t border-amber-500/20 pt-2 leading-relaxed">
                    {msg.thought}
                  </pre>
                )}
              </GlassPanel>
            )}

            {/* Message Body */}
            <div
              className={`rounded-2xl p-3.5 ${
                msg.role === "user"
                  ? "bg-cyan-500/20 text-slate-100 border border-cyan-400/30 rounded-tr-sm"
                  : "bg-[#141B26]/90 text-slate-200 border border-white/10 rounded-tl-sm shadow-[0_4px_16px_rgba(0,0,0,0.3)]"
              }`}
            >
              <div className="whitespace-pre-wrap">{msg.content}</div>
            </div>
          </div>

          {msg.role === "user" && (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl bg-purple-500/20 text-purple-300 border border-purple-500/30">
              <User className="h-3.5 w-3.5" />
            </div>
          )}
        </div>
      ))}

      {/* Live Active Streaming Stream */}
      {isStreaming && (
        <div className="flex gap-3 text-xs leading-relaxed justify-start">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl bg-cyan-500/20 text-cyan-400 border border-cyan-400/40 shadow-[0_0_12px_rgba(56,189,248,0.4)]">
            <Sparkles className="h-3.5 w-3.5 animate-pulse" />
          </div>

          <div className="max-w-2xl space-y-2 flex-1">
            {/* Live Thought Trace */}
            {liveThought && (
              <GlassPanel recipe="pane" className="border-amber-500/30 bg-amber-500/10 p-2.5 rounded-xl">
                <div className="flex items-center justify-between font-mono text-[11px] text-amber-300 border-b border-amber-500/20 pb-1.5">
                  <div className="flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full bg-amber-400 animate-ping" />
                    <span className="font-semibold uppercase tracking-wider">Active Reasoning Trace</span>
                  </div>
                  <div className="flex items-center gap-2 text-slate-400 text-[10px]">
                    <Clock className="h-3 w-3 text-amber-400" />
                    <span>{elapsedSeconds}s</span>
                  </div>
                </div>
                <pre className="mt-2 whitespace-pre-wrap font-mono text-[11px] text-slate-200 leading-relaxed max-h-48 overflow-y-auto">
                  {liveThought}
                </pre>
              </GlassPanel>
            )}

            {/* Live Content Stream */}
            <div className="rounded-2xl rounded-tl-sm bg-[#141B26] p-3.5 text-slate-100 border border-cyan-400/40 shadow-[0_4px_24px_rgba(56,189,248,0.2)]">
              {liveContent ? (
                <div className="whitespace-pre-wrap font-sans">
                  {liveContent}
                  <span className="inline-block w-1.5 h-3.5 bg-cyan-400 ml-1 animate-pulse align-middle" />
                </div>
              ) : (
                <div className="flex items-center gap-2 text-slate-400 font-mono text-[11px]">
                  <Cpu className="h-3.5 w-3.5 text-cyan-400 animate-spin" />
                  <span>Synthesizing token stream...</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      <div ref={scrollEndRef} />
    </div>
  );
};
