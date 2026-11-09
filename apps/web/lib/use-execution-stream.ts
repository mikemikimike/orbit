"use client";

import { useEffect, useRef, useState } from "react";
import { streamExecutionEvents } from "@/lib/sse";
import type { ExecutionEventRecord, ExecutionRecord } from "@/lib/types";

/**
 * Live execution subscription. Merges replayed SSE events with any already-fetched
 * events (deduplicated by `seq`) and keeps the execution snapshot fresh.
 */
export function useExecutionStream(executionId: string | undefined) {
  const [execution, setExecution] = useState<ExecutionRecord | null>(null);
  const [events, setEvents] = useState<ExecutionEventRecord[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const lastSeq = useRef(0);

  useEffect(() => {
    if (!executionId) return;
    const controller = new AbortController();
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
    let stopped = false;

    async function connect() {
      setConnected(true);
      await streamExecutionEvents(executionId!, {
        signal: controller.signal,
        afterSeq: lastSeq.current,
        onEvent: (event) => {
          if (event.event === "snapshot") {
            try {
              setExecution(JSON.parse(event.data) as ExecutionRecord);
            } catch {
              // ignore malformed snapshot
            }
            return;
          }
          if (event.event === "execution.event") {
            try {
              const parsed = JSON.parse(event.data) as ExecutionEventRecord;
              if (typeof event.id === "string") {
                lastSeq.current = Math.max(lastSeq.current, Number(event.id) || 0);
              }
              lastSeq.current = Math.max(lastSeq.current, parsed.seq);
              setEvents((current) => {
                if (current.some((existing) => existing.seq === parsed.seq)) return current;
                return [...current, parsed].sort((a, b) => a.seq - b.seq);
              });
            } catch {
              // ignore malformed event
            }
            return;
          }
          if (event.event === "done") {
            try {
              setExecution(JSON.parse(event.data) as ExecutionRecord);
            } catch {
              // ignore
            }
            setDone(true);
            setConnected(false);
            stopped = true;
            return;
          }
          if (event.event === "error") {
            setError(event.data);
          }
        },
        onError: (streamError) => {
          setError(streamError instanceof Error ? streamError.message : String(streamError));
        },
      });
      setConnected(false);
      if (!stopped && !controller.signal.aborted) {
        reconnectTimer = setTimeout(connect, 1500);
      }
    }

    void connect();
    return () => {
      stopped = true;
      controller.abort();
      if (reconnectTimer) clearTimeout(reconnectTimer);
    };
  }, [executionId]);

  return { execution, events, connected, done, error };
}
