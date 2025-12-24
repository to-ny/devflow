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
} from "../types/agent";

interface ChatState {
  messages: ChatMessage[];
  isLoading: boolean;
  error: string | null;
  streamContent: string;
  streamMessageId: string | null;
}

interface ChatContextValue extends ChatState {
  sendMessage: (content: string) => Promise<void>;
  clearMessages: () => void;
  clearError: () => void;
}

interface ChatProviderProps {
  children: ReactNode;
  projectPath: string | null;
}

const ChatContext = createContext<ChatContextValue | null>(null);

function generateId(): string {
  return crypto.randomUUID();
}

export function ChatProvider({ children, projectPath }: ChatProviderProps) {
  const [state, setState] = useState<ChatState>({
    messages: [],
    isLoading: false,
    error: null,
    streamContent: "",
    streamMessageId: null,
  });

  const isMounted = useRef(true);
  const messagesRef = useRef<ChatMessage[]>([]);

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
    setState({
      messages: [],
      isLoading: false,
      error: null,
      streamContent: "",
      streamMessageId: null,
    });
  }, [projectPath]);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!projectPath || state.isLoading) return;

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
      }));

      try {
        await invoke("agent_send_message", {
          projectPath,
          messages: newMessages,
          systemPrompt: null,
        });
      } catch (err) {
        if (isMounted.current) {
          setState((prev) => ({
            ...prev,
            isLoading: false,
            error: err instanceof Error ? err.message : String(err),
          }));
        }
      }
    },
    [projectPath, state.isLoading],
  );

  const clearMessages = useCallback(() => {
    setState((prev) => ({
      ...prev,
      messages: [],
      streamContent: "",
      streamMessageId: null,
      error: null,
    }));
  }, []);

  const clearError = useCallback(() => {
    setState((prev) => ({ ...prev, error: null }));
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
            const assistantMessage: ChatMessage = {
              id: event.payload.message_id,
              role: "assistant",
              content: prev.streamContent,
            };
            return {
              ...prev,
              messages: [...prev.messages, assistantMessage],
              isLoading: false,
              streamContent: "",
              streamMessageId: null,
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
          }));
        },
      );

      if (cancelled) {
        unlistenChunk();
        unlistenComplete();
        unlistenError();
      } else {
        unlisteners.push(unlistenChunk, unlistenComplete, unlistenError);
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
        clearMessages,
        clearError,
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
