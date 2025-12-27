import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
  ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type {
  AgentUsagePayload,
  UsageTotals,
  MemoryLoadedPayload,
  MemoryWarningPayload,
} from "../types/generated";

export interface MemoryInfo {
  path: string;
  byteLen: number;
  truncated: boolean;
}

interface SessionState {
  sessionUsage: UsageTotals;
  memoryInfo: MemoryInfo | null;
  memoryWarning: string | null;
}

interface SessionContextValue extends SessionState {
  resetSession: () => void;
  clearMemoryWarning: () => void;
}

interface SessionProviderProps {
  children: ReactNode;
  projectPath: string | null;
}

const SessionContext = createContext<SessionContextValue | null>(null);

const INITIAL_USAGE: UsageTotals = { input_tokens: 0, output_tokens: 0 };

export function SessionProvider({
  children,
  projectPath,
}: SessionProviderProps) {
  const [state, setState] = useState<SessionState>({
    sessionUsage: INITIAL_USAGE,
    memoryInfo: null,
    memoryWarning: null,
  });

  const isMounted = useRef(true);

  useEffect(() => {
    isMounted.current = true;
    return () => {
      isMounted.current = false;
    };
  }, []);

  // Reset session when project changes
  useEffect(() => {
    invoke("reset_session_usage").catch(() => {});
    setState({
      sessionUsage: INITIAL_USAGE,
      memoryInfo: null,
      memoryWarning: null,
    });
  }, [projectPath]);

  const resetSession = useCallback(() => {
    invoke("reset_session_usage").catch(() => {});
    setState((prev) => ({
      ...prev,
      sessionUsage: INITIAL_USAGE,
    }));
  }, []);

  const clearMemoryWarning = useCallback(() => {
    setState((prev) => ({ ...prev, memoryWarning: null }));
  }, []);

  // Set up event listeners
  useEffect(() => {
    let cancelled = false;
    const unlisteners: (() => void)[] = [];

    async function setupListeners() {
      const unlistenUsage = await listen<AgentUsagePayload>(
        "agent-usage",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            sessionUsage: {
              input_tokens: event.payload.input_tokens,
              output_tokens: event.payload.output_tokens,
            },
          }));
        },
      );

      const unlistenMemoryLoaded = await listen<MemoryLoadedPayload>(
        "memory-loaded",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            memoryInfo: {
              path: event.payload.path,
              byteLen: event.payload.byte_len,
              truncated: event.payload.truncated,
            },
            memoryWarning: null,
          }));
        },
      );

      const unlistenMemoryWarning = await listen<MemoryWarningPayload>(
        "memory-warning",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            memoryWarning: event.payload.message,
          }));
        },
      );

      if (cancelled) {
        unlistenUsage();
        unlistenMemoryLoaded();
        unlistenMemoryWarning();
      } else {
        unlisteners.push(
          unlistenUsage,
          unlistenMemoryLoaded,
          unlistenMemoryWarning,
        );
      }
    }

    setupListeners();

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  return (
    <SessionContext.Provider
      value={{
        ...state,
        resetSession,
        clearMemoryWarning,
      }}
    >
      {children}
    </SessionContext.Provider>
  );
}

export function useSession() {
  const context = useContext(SessionContext);
  if (!context) {
    throw new Error("useSession must be used within a SessionProvider");
  }
  return context;
}
