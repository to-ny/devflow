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
  ChatMessage,
  AgentChunkPayload,
  AgentCompletePayload,
  AgentErrorPayload,
  AgentStatusPayload,
  AgentCancelledPayload,
  AgentStatus,
  PlanReadyPayload,
  ToolStartPayload,
  ToolEndPayload,
} from "../types/agent";

const PROMPT_HISTORY_KEY = "devflow_prompt_history";
const MAX_PROMPT_HISTORY = 50;
const MAX_QUEUED_MESSAGES = 10;

export interface ToolExecution {
  toolUseId: string;
  toolName: string;
  toolInput: unknown;
  output?: string;
  isError?: boolean;
  isComplete: boolean;
}

export interface QueuedMessage {
  id: string;
  content: string;
  status: "pending" | "sending" | "sent";
}

interface ChatState {
  messages: ChatMessage[];
  isLoading: boolean;
  error: string | null;
  streamContent: string;
  streamMessageId: string | null;
  toolExecutions: ToolExecution[];
  agentStatus: AgentStatus;
  statusText: string;
  messageQueue: QueuedMessage[];
  promptHistory: string[];
  pendingPlan: string | null;
}

interface ChatContextValue extends ChatState {
  sendMessage: (content: string) => Promise<void>;
  cancelRequest: () => Promise<void>;
  clearMessages: () => void;
  clearError: () => void;
  addToQueue: (content: string) => string;
  removeFromQueue: (id: string) => void;
  updateQueuedMessage: (id: string, content: string) => void;
  clearPromptHistory: () => void;
  approvePlan: () => Promise<void>;
  rejectPlan: (reason?: string) => Promise<void>;
}

interface ChatProviderProps {
  children: ReactNode;
  projectPath: string | null;
}

const ChatContext = createContext<ChatContextValue | null>(null);

function generateId(): string {
  return crypto.randomUUID();
}

// Partial state for clearing pending plan - use spread in state updates
const CLEAR_PENDING_PLAN = { pendingPlan: null } as const;

function loadPromptHistory(): string[] {
  try {
    const stored = localStorage.getItem(PROMPT_HISTORY_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed)) {
        return parsed.slice(0, MAX_PROMPT_HISTORY);
      }
    }
  } catch {
    // Ignore parse errors
  }
  return [];
}

function savePromptHistory(history: string[]): void {
  try {
    localStorage.setItem(
      PROMPT_HISTORY_KEY,
      JSON.stringify(history.slice(0, MAX_PROMPT_HISTORY)),
    );
  } catch {
    // Ignore storage errors
  }
}

function addToPromptHistory(history: string[], prompt: string): string[] {
  // Remove duplicates and add to front
  const filtered = history.filter((p) => p !== prompt);
  const newHistory = [prompt, ...filtered].slice(0, MAX_PROMPT_HISTORY);
  savePromptHistory(newHistory);
  return newHistory;
}

export function ChatProvider({ children, projectPath }: ChatProviderProps) {
  const [state, setState] = useState<ChatState>(() => ({
    messages: [],
    isLoading: false,
    error: null,
    streamContent: "",
    streamMessageId: null,
    toolExecutions: [],
    agentStatus: "idle" as AgentStatus,
    statusText: "",
    messageQueue: [],
    promptHistory: loadPromptHistory(),
    ...CLEAR_PENDING_PLAN,
  }));

  const isMounted = useRef(true);
  const messagesRef = useRef<ChatMessage[]>([]);
  const isProcessingQueue = useRef(false);

  useEffect(() => {
    messagesRef.current = state.messages;
  }, [state.messages]);

  useEffect(() => {
    isMounted.current = true;
    return () => {
      isMounted.current = false;
    };
  }, []);

  // Reset chat when project changes
  useEffect(() => {
    setState((prev) => ({
      ...prev,
      messages: [],
      isLoading: false,
      error: null,
      streamContent: "",
      streamMessageId: null,
      toolExecutions: [],
      agentStatus: "idle" as AgentStatus,
      statusText: "",
      messageQueue: [],
      ...CLEAR_PENDING_PLAN,
    }));
  }, [projectPath]);

  const sendMessageInternal = useCallback(
    async (content: string) => {
      if (!projectPath) return;

      const userMessage: ChatMessage = {
        id: generateId(),
        role: "user",
        content,
      };
      const newMessages = [...messagesRef.current, userMessage];

      setState((prev) => ({
        ...prev,
        messages: newMessages,
        isLoading: true,
        error: null,
        streamContent: "",
        streamMessageId: null,
        toolExecutions: [],
        agentStatus: "sending" as AgentStatus,
        statusText: "Sending...",
        promptHistory: addToPromptHistory(prev.promptHistory, content),
      }));

      try {
        await invoke("agent_send_message", {
          projectPath,
          messages: newMessages,
          systemPrompt: null,
        });
      } catch (err) {
        if (isMounted.current) {
          const errorMessage = err instanceof Error ? err.message : String(err);
          // Don't show error for cancellation
          if (!errorMessage.includes("cancelled")) {
            setState((prev) => ({
              ...prev,
              isLoading: false,
              error: errorMessage,
              agentStatus: "error" as AgentStatus,
            }));
          }
        }
      }
    },
    [projectPath],
  );

  // Process queue when not loading
  useEffect(() => {
    async function processQueue() {
      if (
        isProcessingQueue.current ||
        state.isLoading ||
        state.messageQueue.length === 0 ||
        !projectPath
      ) {
        return;
      }

      const pendingMessage = state.messageQueue.find(
        (m) => m.status === "pending",
      );
      if (!pendingMessage) return;

      isProcessingQueue.current = true;

      // Mark as sending
      setState((prev) => ({
        ...prev,
        messageQueue: prev.messageQueue.map((m) =>
          m.id === pendingMessage.id ? { ...m, status: "sending" as const } : m,
        ),
      }));

      // Send the message
      await sendMessageInternal(pendingMessage.content);

      // Mark as sent and remove from queue
      setState((prev) => ({
        ...prev,
        messageQueue: prev.messageQueue.filter(
          (m) => m.id !== pendingMessage.id,
        ),
      }));

      isProcessingQueue.current = false;
    }

    processQueue();
  }, [state.isLoading, state.messageQueue, projectPath, sendMessageInternal]);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!projectPath || !content.trim()) return;

      // If already loading, add to queue
      if (state.isLoading) {
        if (state.messageQueue.length < MAX_QUEUED_MESSAGES) {
          setState((prev) => ({
            ...prev,
            messageQueue: [
              ...prev.messageQueue,
              { id: generateId(), content: content.trim(), status: "pending" },
            ],
          }));
        }
        return;
      }

      await sendMessageInternal(content.trim());
    },
    [
      projectPath,
      state.isLoading,
      state.messageQueue.length,
      sendMessageInternal,
    ],
  );

  const cancelRequest = useCallback(async () => {
    try {
      await invoke("agent_cancel");
      if (isMounted.current) {
        setState((prev) => {
          // Save partial response as a message if there's any content
          const hasContent =
            prev.streamContent || prev.toolExecutions.length > 0;

          if (!hasContent) {
            return {
              ...prev,
              isLoading: false,
              agentStatus: "cancelled" as AgentStatus,
              statusText: "Cancelled",
              ...CLEAR_PENDING_PLAN,
            };
          }

          // Convert tool executions for the message
          const toolExecutions =
            prev.toolExecutions.length > 0
              ? prev.toolExecutions.map((exec) => ({
                  tool_use_id: exec.toolUseId,
                  tool_name: exec.toolName,
                  tool_input: exec.toolInput,
                  output: exec.output ?? null,
                  is_error: exec.isError ?? null,
                }))
              : undefined;

          const cancelledMessage: ChatMessage = {
            id: generateId(),
            role: "assistant",
            content: prev.streamContent
              ? prev.streamContent + "\n\n*[Cancelled by user]*"
              : "*[Cancelled by user]*",
            tool_executions: toolExecutions,
          };

          return {
            ...prev,
            messages: [...prev.messages, cancelledMessage],
            isLoading: false,
            streamContent: "",
            toolExecutions: [],
            agentStatus: "cancelled" as AgentStatus,
            statusText: "Cancelled",
            ...CLEAR_PENDING_PLAN,
          };
        });
      }
    } catch {
      // Ignore cancel errors
    }
  }, []);

  const clearMessages = useCallback(() => {
    setState((prev) => ({
      ...prev,
      messages: [],
      streamContent: "",
      streamMessageId: null,
      error: null,
      toolExecutions: [],
      agentStatus: "idle" as AgentStatus,
      statusText: "",
      messageQueue: [],
    }));
  }, []);

  const clearError = useCallback(() => {
    setState((prev) => ({ ...prev, error: null }));
  }, []);

  const addToQueue = useCallback((content: string): string => {
    const id = generateId();
    setState((prev) => {
      if (prev.messageQueue.length >= MAX_QUEUED_MESSAGES) {
        return prev;
      }
      return {
        ...prev,
        messageQueue: [
          ...prev.messageQueue,
          { id, content: content.trim(), status: "pending" },
        ],
      };
    });
    return id;
  }, []);

  const removeFromQueue = useCallback((id: string) => {
    setState((prev) => ({
      ...prev,
      messageQueue: prev.messageQueue.filter((m) => m.id !== id),
    }));
  }, []);

  const updateQueuedMessage = useCallback((id: string, content: string) => {
    setState((prev) => ({
      ...prev,
      messageQueue: prev.messageQueue.map((m) =>
        m.id === id && m.status === "pending" ? { ...m, content } : m,
      ),
    }));
  }, []);

  const clearPromptHistory = useCallback(() => {
    localStorage.removeItem(PROMPT_HISTORY_KEY);
    setState((prev) => ({ ...prev, promptHistory: [] }));
  }, []);

  const approvePlan = useCallback(async () => {
    try {
      await invoke("agent_approve_plan");
      setState((prev) => ({ ...prev, ...CLEAR_PENDING_PLAN }));
    } catch (error) {
      setState((prev) => ({
        ...prev,
        ...CLEAR_PENDING_PLAN,
        error: `Failed to approve plan: ${error}`,
      }));
    }
  }, []);

  const rejectPlan = useCallback(async (reason?: string) => {
    try {
      await invoke("agent_reject_plan", { reason });
      setState((prev) => ({ ...prev, ...CLEAR_PENDING_PLAN }));
    } catch (error) {
      setState((prev) => ({
        ...prev,
        ...CLEAR_PENDING_PLAN,
        error: `Failed to reject plan: ${error}`,
      }));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: (() => void)[] = [];

    async function setupListeners() {
      const unlistenChunk = await listen<AgentChunkPayload>(
        "agent-chunk",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            streamContent: prev.streamContent + event.payload.delta,
          }));
        },
      );

      const unlistenComplete = await listen<AgentCompletePayload>(
        "agent-complete",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => {
            // Convert tool executions to the format expected by ChatMessage
            // Note: Generated types use null, not undefined
            const toolExecutions =
              prev.toolExecutions.length > 0
                ? prev.toolExecutions.map((exec) => ({
                    tool_use_id: exec.toolUseId,
                    tool_name: exec.toolName,
                    tool_input: exec.toolInput,
                    output: exec.output ?? null,
                    is_error: exec.isError ?? null,
                  }))
                : undefined;

            const assistantMessage: ChatMessage = {
              id: event.payload.message_id,
              role: "assistant",
              content: prev.streamContent,
              tool_executions: toolExecutions,
            };
            return {
              ...prev,
              messages: [...prev.messages, assistantMessage],
              isLoading: false,
              streamContent: "",
              streamMessageId: null,
              toolExecutions: [],
              agentStatus: "idle" as AgentStatus,
              statusText: "",
            };
          });
        },
      );

      const unlistenError = await listen<AgentErrorPayload>(
        "agent-error",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            isLoading: false,
            error: event.payload.error,
            streamContent: "",
            streamMessageId: null,
            agentStatus: "error" as AgentStatus,
          }));
        },
      );

      const unlistenStatus = await listen<AgentStatusPayload>(
        "agent-status",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            agentStatus: event.payload.status,
            statusText: event.payload.status_text,
          }));
        },
      );

      const unlistenCancelled = await listen<AgentCancelledPayload>(
        "agent-cancelled",
        () => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => {
            // If already handled by cancelRequest, just update status
            if (!prev.isLoading) {
              return prev;
            }

            // Save partial response as a message if there's any content
            const hasContent =
              prev.streamContent || prev.toolExecutions.length > 0;

            if (!hasContent) {
              return {
                ...prev,
                isLoading: false,
                agentStatus: "cancelled" as AgentStatus,
                statusText: "Cancelled",
                ...CLEAR_PENDING_PLAN,
              };
            }

            // Convert tool executions for the message
            const toolExecutions =
              prev.toolExecutions.length > 0
                ? prev.toolExecutions.map((exec) => ({
                    tool_use_id: exec.toolUseId,
                    tool_name: exec.toolName,
                    tool_input: exec.toolInput,
                    output: exec.output ?? null,
                    is_error: exec.isError ?? null,
                  }))
                : undefined;

            const cancelledMessage: ChatMessage = {
              id: generateId(),
              role: "assistant",
              content: prev.streamContent
                ? prev.streamContent + "\n\n*[Cancelled by user]*"
                : "*[Cancelled by user]*",
              tool_executions: toolExecutions,
            };

            return {
              ...prev,
              messages: [...prev.messages, cancelledMessage],
              isLoading: false,
              streamContent: "",
              toolExecutions: [],
              agentStatus: "cancelled" as AgentStatus,
              statusText: "Cancelled",
              ...CLEAR_PENDING_PLAN,
            };
          });
        },
      );

      const unlistenToolStart = await listen<ToolStartPayload>(
        "agent-tool-start",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            toolExecutions: [
              ...prev.toolExecutions,
              {
                toolUseId: event.payload.tool_use_id,
                toolName: event.payload.tool_name,
                toolInput: event.payload.tool_input,
                isComplete: false,
              },
            ],
          }));
        },
      );

      const unlistenToolEnd = await listen<ToolEndPayload>(
        "agent-tool-end",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            toolExecutions: prev.toolExecutions.map((exec) =>
              exec.toolUseId === event.payload.tool_use_id
                ? {
                    ...exec,
                    output: event.payload.output,
                    isError: event.payload.is_error,
                    isComplete: true,
                  }
                : exec,
            ),
          }));
        },
      );

      const unlistenPlanReady = await listen<PlanReadyPayload>(
        "agent-plan-ready",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            pendingPlan: event.payload.plan,
            streamContent: "",
          }));
        },
      );

      if (cancelled) {
        unlistenChunk();
        unlistenComplete();
        unlistenError();
        unlistenStatus();
        unlistenCancelled();
        unlistenToolStart();
        unlistenToolEnd();
        unlistenPlanReady();
      } else {
        unlisteners.push(
          unlistenChunk,
          unlistenComplete,
          unlistenError,
          unlistenStatus,
          unlistenCancelled,
          unlistenToolStart,
          unlistenToolEnd,
          unlistenPlanReady,
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
    <ChatContext.Provider
      value={{
        ...state,
        sendMessage,
        cancelRequest,
        clearMessages,
        clearError,
        addToQueue,
        removeFromQueue,
        updateQueuedMessage,
        clearPromptHistory,
        approvePlan,
        rejectPlan,
      }}
    >
      {children}
    </ChatContext.Provider>
  );
}

export function useChat() {
  const context = useContext(ChatContext);
  if (!context) {
    throw new Error("useChat must be used within a ChatProvider");
  }
  return context;
}
