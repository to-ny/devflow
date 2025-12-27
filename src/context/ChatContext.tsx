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
  ChatContentBlock,
  AgentChunkPayload,
  AgentCompletePayload,
  AgentErrorPayload,
  AgentStatusPayload,
  AgentCancelledPayload,
  AgentStatus,
  PlanReadyPayload,
  ToolStartPayload,
  ToolEndPayload,
  ContentBlockStartPayload,
} from "../types/agent";
import type { AgentUsagePayload, UsageTotals } from "../types/generated";

const PROMPT_HISTORY_KEY = "devflow_prompt_history";
const MAX_PROMPT_HISTORY = 50;
const MAX_QUEUED_MESSAGES = 10;

export interface StreamingBlock {
  blockIndex: number;
  type: "text" | "tool_use";
  text?: string;
  toolUseId?: string;
  toolName?: string;
  toolInput?: unknown;
  output?: string;
  isError?: boolean;
  isComplete?: boolean;
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
  streamBlocks: StreamingBlock[];
  streamMessageId: string | null;
  agentStatus: AgentStatus;
  statusText: string;
  messageQueue: QueuedMessage[];
  promptHistory: string[];
  pendingPlan: string | null;
  sessionUsage: UsageTotals;
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

const CLEAR_PENDING_PLAN = { pendingPlan: null } as const;
const INITIAL_USAGE: UsageTotals = { input_tokens: 0, output_tokens: 0 };

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
    streamBlocks: [],
    streamMessageId: null,
    agentStatus: "idle" as AgentStatus,
    statusText: "",
    messageQueue: [],
    promptHistory: loadPromptHistory(),
    ...CLEAR_PENDING_PLAN,
    sessionUsage: INITIAL_USAGE,
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
    // Fire-and-forget: backend may not be ready yet
    invoke("reset_session_usage").catch(() => {});
    setState((prev) => ({
      ...prev,
      messages: [],
      isLoading: false,
      error: null,
      streamBlocks: [],
      streamMessageId: null,
      agentStatus: "idle" as AgentStatus,
      statusText: "",
      messageQueue: [],
      ...CLEAR_PENDING_PLAN,
      sessionUsage: INITIAL_USAGE,
    }));
  }, [projectPath]);

  const sendMessageInternal = useCallback(
    async (content: string) => {
      if (!projectPath) return;

      const userMessage: ChatMessage = {
        id: generateId(),
        role: "user",
        content_blocks: [{ type: "text", text: content }],
      };
      const newMessages = [...messagesRef.current, userMessage];

      setState((prev) => ({
        ...prev,
        messages: newMessages,
        isLoading: true,
        error: null,
        streamBlocks: [],
        streamMessageId: null,
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
          if (prev.streamBlocks.length === 0) {
            return {
              ...prev,
              isLoading: false,
              agentStatus: "cancelled" as AgentStatus,
              statusText: "Cancelled",
              ...CLEAR_PENDING_PLAN,
            };
          }

          // Convert streaming blocks to content blocks
          const contentBlocks: ChatContentBlock[] = prev.streamBlocks
            .sort((a, b) => a.blockIndex - b.blockIndex)
            .map((block): ChatContentBlock => {
              if (block.type === "text") {
                const text = block.text || "";
                return {
                  type: "text",
                  text: text + "\n\n*[Cancelled by user]*",
                };
              } else {
                return {
                  type: "tool_use",
                  tool_use_id: block.toolUseId!,
                  tool_name: block.toolName!,
                  tool_input: block.toolInput,
                  output: block.output ?? null,
                  is_error: block.isError ?? null,
                };
              }
            });

          // If no text block exists, add cancellation message
          const hasTextBlock = contentBlocks.some((b) => b.type === "text");
          if (!hasTextBlock) {
            contentBlocks.push({ type: "text", text: "*[Cancelled by user]*" });
          }

          const cancelledMessage: ChatMessage = {
            id: generateId(),
            role: "assistant",
            content_blocks: contentBlocks,
          };

          return {
            ...prev,
            messages: [...prev.messages, cancelledMessage],
            isLoading: false,
            streamBlocks: [],
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
    // Fire-and-forget: non-critical operation
    invoke("reset_session_usage").catch(() => {});
    setState((prev) => ({
      ...prev,
      messages: [],
      streamBlocks: [],
      streamMessageId: null,
      error: null,
      agentStatus: "idle" as AgentStatus,
      statusText: "",
      messageQueue: [],
      sessionUsage: INITIAL_USAGE,
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
      const approved = await invoke<boolean>("agent_approve_plan");
      if (!approved) {
        setState((prev) => ({
          ...prev,
          ...CLEAR_PENDING_PLAN,
          error: "Plan approval failed - please try again",
        }));
        return;
      }
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
      const rejected = await invoke<boolean>("agent_reject_plan", { reason });
      if (!rejected) {
        setState((prev) => ({
          ...prev,
          ...CLEAR_PENDING_PLAN,
          error: "Plan rejection failed - please try again",
        }));
        return;
      }
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
      // Listen for content block start to create new streaming blocks
      const unlistenBlockStart = await listen<ContentBlockStartPayload>(
        "agent-content-block-start",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => {
            const newBlock: StreamingBlock = {
              blockIndex: event.payload.block_index,
              type:
                event.payload.block_type.type === "text" ? "text" : "tool_use",
              text: event.payload.block_type.type === "text" ? "" : undefined,
              toolUseId:
                event.payload.block_type.type === "tool_use"
                  ? event.payload.block_type.tool_use_id
                  : undefined,
              toolName:
                event.payload.block_type.type === "tool_use"
                  ? event.payload.block_type.tool_name
                  : undefined,
            };
            return {
              ...prev,
              streamBlocks: [...prev.streamBlocks, newBlock],
            };
          });
        },
      );

      const unlistenChunk = await listen<AgentChunkPayload>(
        "agent-chunk",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => {
            const blocks = [...prev.streamBlocks];
            const blockIdx = blocks.findIndex(
              (b) => b.blockIndex === event.payload.block_index,
            );
            if (blockIdx !== -1 && blocks[blockIdx].type === "text") {
              blocks[blockIdx] = {
                ...blocks[blockIdx],
                text: (blocks[blockIdx].text || "") + event.payload.delta,
              };
            }
            return { ...prev, streamBlocks: blocks };
          });
        },
      );

      const unlistenComplete = await listen<AgentCompletePayload>(
        "agent-complete",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => {
            // Convert streaming blocks to ChatContentBlocks
            const contentBlocks: ChatContentBlock[] = prev.streamBlocks
              .sort((a, b) => a.blockIndex - b.blockIndex)
              .map((block): ChatContentBlock => {
                if (block.type === "text") {
                  return { type: "text", text: block.text || "" };
                } else {
                  return {
                    type: "tool_use",
                    tool_use_id: block.toolUseId!,
                    tool_name: block.toolName!,
                    tool_input: block.toolInput,
                    output: block.output ?? null,
                    is_error: block.isError ?? null,
                  };
                }
              });

            const assistantMessage: ChatMessage = {
              id: event.payload.message_id,
              role: "assistant",
              content_blocks: contentBlocks,
            };
            return {
              ...prev,
              messages: [...prev.messages, assistantMessage],
              isLoading: false,
              streamBlocks: [],
              streamMessageId: null,
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
            streamBlocks: [],
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
            if (prev.streamBlocks.length === 0) {
              return {
                ...prev,
                isLoading: false,
                agentStatus: "cancelled" as AgentStatus,
                statusText: "Cancelled",
                ...CLEAR_PENDING_PLAN,
              };
            }

            // Convert streaming blocks to content blocks
            const contentBlocks: ChatContentBlock[] = prev.streamBlocks
              .sort((a, b) => a.blockIndex - b.blockIndex)
              .map((block): ChatContentBlock => {
                if (block.type === "text") {
                  const text = block.text || "";
                  return {
                    type: "text",
                    text: text + "\n\n*[Cancelled by user]*",
                  };
                } else {
                  return {
                    type: "tool_use",
                    tool_use_id: block.toolUseId!,
                    tool_name: block.toolName!,
                    tool_input: block.toolInput,
                    output: block.output ?? null,
                    is_error: block.isError ?? null,
                  };
                }
              });

            // If no text block exists, add cancellation message
            const hasTextBlock = contentBlocks.some((b) => b.type === "text");
            if (!hasTextBlock) {
              contentBlocks.push({
                type: "text",
                text: "*[Cancelled by user]*",
              });
            }

            const cancelledMessage: ChatMessage = {
              id: generateId(),
              role: "assistant",
              content_blocks: contentBlocks,
            };

            return {
              ...prev,
              messages: [...prev.messages, cancelledMessage],
              isLoading: false,
              streamBlocks: [],
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
          setState((prev) => {
            const blocks = [...prev.streamBlocks];
            const blockIdx = blocks.findIndex(
              (b) => b.blockIndex === event.payload.block_index,
            );
            if (blockIdx !== -1) {
              blocks[blockIdx] = {
                ...blocks[blockIdx],
                toolUseId: event.payload.tool_use_id,
                toolName: event.payload.tool_name,
                toolInput: event.payload.tool_input,
                isComplete: false,
              };
            }
            return { ...prev, streamBlocks: blocks };
          });
        },
      );

      const unlistenToolEnd = await listen<ToolEndPayload>(
        "agent-tool-end",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => {
            const blocks = [...prev.streamBlocks];
            const blockIdx = blocks.findIndex(
              (b) => b.blockIndex === event.payload.block_index,
            );
            if (blockIdx !== -1) {
              blocks[blockIdx] = {
                ...blocks[blockIdx],
                output: event.payload.output,
                isError: event.payload.is_error,
                isComplete: true,
              };
            }
            return { ...prev, streamBlocks: blocks };
          });
        },
      );

      const unlistenPlanReady = await listen<PlanReadyPayload>(
        "agent-plan-ready",
        (event) => {
          if (cancelled || !isMounted.current) return;
          setState((prev) => ({
            ...prev,
            pendingPlan: event.payload.plan,
            streamBlocks: [],
          }));
        },
      );

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

      if (cancelled) {
        unlistenBlockStart();
        unlistenChunk();
        unlistenComplete();
        unlistenError();
        unlistenStatus();
        unlistenCancelled();
        unlistenToolStart();
        unlistenToolEnd();
        unlistenPlanReady();
        unlistenUsage();
      } else {
        unlisteners.push(
          unlistenBlockStart,
          unlistenChunk,
          unlistenComplete,
          unlistenError,
          unlistenStatus,
          unlistenCancelled,
          unlistenToolStart,
          unlistenToolEnd,
          unlistenPlanReady,
          unlistenUsage,
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
