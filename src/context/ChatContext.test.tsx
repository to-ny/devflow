import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { ReactNode } from "react";
import { SessionProvider } from "./SessionContext";
import { ChatProvider, useChat } from "./ChatContext";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AgentChunkPayload,
  AgentCompletePayload,
  AgentErrorPayload,
  AgentCancelledPayload,
  ToolStartPayload,
  ToolEndPayload,
  PlanReadyPayload,
  ContentBlockStartPayload,
} from "../types/agent";

type EventCallback<T> = (event: { payload: T }) => void;
const eventListeners: Map<string, EventCallback<unknown>> = new Map();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, callback: EventCallback<unknown>) => {
    eventListeners.set(eventName, callback);
    return Promise.resolve(() => {
      eventListeners.delete(eventName);
    });
  }),
}));

function simulateEvent<T>(eventName: string, payload: T) {
  const callback = eventListeners.get(eventName);
  if (callback) {
    callback({ payload });
  }
}

// Wrapper component for testing
function createWrapper(projectPath: string | null) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <SessionProvider projectPath={projectPath}>
        <ChatProvider projectPath={projectPath}>{children}</ChatProvider>
      </SessionProvider>
    );
  };
}

describe("useChat", () => {
  it("throws error when used outside ChatProvider", () => {
    expect(() => renderHook(() => useChat())).toThrow(
      "useChat must be used within a ChatProvider",
    );
  });
});

describe("ChatProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventListeners.clear();
    localStorage.clear();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  afterEach(() => {
    eventListeners.clear();
  });

  describe("initial state", () => {
    it("provides initial idle state", () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      expect(result.current.messages).toEqual([]);
      expect(result.current.isLoading).toBe(false);
      expect(result.current.error).toBeNull();
      expect(result.current.streamBlocks).toEqual([]);
      expect(result.current.agentStatus).toBe("idle");
      expect(result.current.messageQueue).toEqual([]);
      expect(result.current.pendingPlan).toBeNull();
    });

    it("sets up event listeners on mount", async () => {
      renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(listen).toHaveBeenCalledWith(
          "agent-chunk",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-complete",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-error",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-status",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-cancelled",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-tool-start",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-tool-end",
          expect.any(Function),
        );
        expect(listen).toHaveBeenCalledWith(
          "agent-plan-ready",
          expect.any(Function),
        );
      });
    });
  });

  describe("sendMessage", () => {
    it("invokes correct Tauri command with correct params", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.sendMessage("Hello, AI!");
      });

      expect(invoke).toHaveBeenCalledWith("agent_send_message", {
        projectPath: "/test/project",
        messages: expect.arrayContaining([
          expect.objectContaining({
            role: "user",
            content_blocks: [{ type: "text", text: "Hello, AI!" }],
          }),
        ]),
        systemPrompt: null,
      });
    });

    it("adds user message to messages array", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.sendMessage("Test message");
      });

      expect(result.current.messages).toHaveLength(1);
      expect(result.current.messages[0].role).toBe("user");
      expect(result.current.messages[0].content_blocks).toEqual([
        { type: "text", text: "Test message" },
      ]);
    });

    it("sets loading state to true while sending", async () => {
      vi.mocked(invoke).mockImplementation(
        () => new Promise(() => {}), // Never resolves
      );

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      act(() => {
        result.current.sendMessage("Test message");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
        expect(result.current.agentStatus).toBe("sending");
      });
    });

    it("does not send empty messages", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.sendMessage("   ");
      });

      expect(invoke).not.toHaveBeenCalledWith(
        "agent_send_message",
        expect.anything(),
      );
      expect(result.current.messages).toHaveLength(0);
    });

    it("does not send when projectPath is null", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper(null),
      });

      await act(async () => {
        await result.current.sendMessage("Test");
      });

      expect(invoke).not.toHaveBeenCalledWith(
        "agent_send_message",
        expect.anything(),
      );
    });

    it("adds to prompt history when sending message", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.sendMessage("Test prompt");
      });

      expect(result.current.promptHistory).toContain("Test prompt");
    });
  });

  describe("message queuing", () => {
    it("queues message when agent is already processing", async () => {
      vi.mocked(invoke).mockImplementation(
        () => new Promise(() => {}), // Never resolves
      );

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      // Start first message
      act(() => {
        result.current.sendMessage("First message");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      // Try to send second message while first is loading
      await act(async () => {
        await result.current.sendMessage("Second message");
      });

      expect(result.current.messageQueue).toHaveLength(1);
      expect(result.current.messageQueue[0].content).toBe("Second message");
      expect(result.current.messageQueue[0].status).toBe("pending");
    });

    it("limits queue to maximum size", async () => {
      vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      act(() => {
        result.current.sendMessage("First message");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      // Try to queue more than max (10)
      for (let i = 0; i < 12; i++) {
        await act(async () => {
          await result.current.sendMessage(`Queued ${i}`);
        });
      }

      expect(result.current.messageQueue.length).toBeLessThanOrEqual(10);
    });

    it("removes message from queue", async () => {
      vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      act(() => {
        result.current.sendMessage("First message");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      await act(async () => {
        await result.current.sendMessage("Queued message");
      });

      const queuedId = result.current.messageQueue[0].id;

      act(() => {
        result.current.removeFromQueue(queuedId);
      });

      expect(result.current.messageQueue).toHaveLength(0);
    });
  });

  describe("event handling - agent-chunk", () => {
    it("accumulates stream content on chunk events", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-chunk")).toBe(true);
      });

      // First, start a text block
      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: { type: "text" },
        });
      });

      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "Hello ",
          block_index: 0,
        });
      });

      expect(result.current.streamBlocks[0].text).toBe("Hello ");

      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "World!",
          block_index: 0,
        });
      });

      expect(result.current.streamBlocks[0].text).toBe("Hello World!");
    });
  });

  describe("event handling - agent-complete", () => {
    it("creates assistant message on complete event", async () => {
      vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-complete")).toBe(true);
      });

      // Start a message to set loading state
      act(() => {
        result.current.sendMessage("Test");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      // Simulate streaming - first start a text block
      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: { type: "text" },
        });
      });

      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "Response content",
          block_index: 0,
        });
      });

      // Complete the message
      act(() => {
        simulateEvent<AgentCompletePayload>("agent-complete", {
          message_id: "msg-123",
          stop_reason: "end_turn",
        });
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(result.current.messages).toHaveLength(2); // user + assistant
      expect(result.current.messages[1].role).toBe("assistant");
      expect(result.current.messages[1].content_blocks).toEqual([
        { type: "text", text: "Response content" },
      ]);
      expect(result.current.streamBlocks).toEqual([]);
      expect(result.current.agentStatus).toBe("idle");
    });
  });

  describe("event handling - agent-error", () => {
    it("sets error state on error event", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-error")).toBe(true);
      });

      act(() => {
        simulateEvent<AgentErrorPayload>("agent-error", {
          error: "API rate limit exceeded",
        });
      });

      expect(result.current.error).toBe("API rate limit exceeded");
      expect(result.current.isLoading).toBe(false);
      expect(result.current.agentStatus).toBe("error");
    });
  });

  describe("event handling - agent-cancelled", () => {
    it("handles cancellation event gracefully", async () => {
      vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-cancelled")).toBe(true);
      });

      // Start a request
      act(() => {
        result.current.sendMessage("Test");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      // Simulate cancellation
      act(() => {
        simulateEvent<AgentCancelledPayload>("agent-cancelled", {
          reason: "User cancelled",
        });
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });
      expect(result.current.agentStatus).toBe("cancelled");
    });

    it("saves partial content on cancellation", async () => {
      vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-cancelled")).toBe(true);
      });

      // Start a request
      act(() => {
        result.current.sendMessage("Test");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      // Start a text block and stream some content
      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: { type: "text" },
        });
      });

      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "Partial response",
          block_index: 0,
        });
      });

      // Cancel
      act(() => {
        simulateEvent<AgentCancelledPayload>("agent-cancelled", {
          reason: "User cancelled",
        });
      });

      await waitFor(() => {
        expect(result.current.messages.length).toBe(2); // user + partial assistant
      });
      const textBlock = result.current.messages[1].content_blocks.find(
        (b) => b.type === "text",
      );
      expect(textBlock).toBeDefined();
      if (textBlock && textBlock.type === "text") {
        expect(textBlock.text).toContain("Partial response");
        expect(textBlock.text).toContain("[Cancelled by user]");
      }
    });
  });

  describe("tool execution tracking", () => {
    it("updates tool block on tool-start event", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-tool-start")).toBe(true);
      });

      // First start the tool block
      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: {
            type: "tool_use",
            tool_use_id: "tool-1",
            tool_name: "read_file",
          },
        });
      });

      act(() => {
        simulateEvent<ToolStartPayload>("agent-tool-start", {
          tool_use_id: "tool-1",
          tool_name: "read_file",
          tool_input: { path: "/test/file.txt" },
          block_index: 0,
        });
      });

      expect(result.current.streamBlocks).toHaveLength(1);
      expect(result.current.streamBlocks[0].toolUseId).toBe("tool-1");
      expect(result.current.streamBlocks[0].toolName).toBe("read_file");
      expect(result.current.streamBlocks[0].isComplete).toBe(false);
    });

    it("completes tool block on tool-end event", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-tool-end")).toBe(true);
      });

      // Start tool block
      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: {
            type: "tool_use",
            tool_use_id: "tool-1",
            tool_name: "bash",
          },
        });
      });

      act(() => {
        simulateEvent<ToolStartPayload>("agent-tool-start", {
          tool_use_id: "tool-1",
          tool_name: "bash",
          tool_input: { command: "ls" },
          block_index: 0,
        });
      });

      // End tool
      act(() => {
        simulateEvent<ToolEndPayload>("agent-tool-end", {
          tool_use_id: "tool-1",
          output: "file1.txt\nfile2.txt",
          is_error: false,
          block_index: 0,
        });
      });

      expect(result.current.streamBlocks[0].isComplete).toBe(true);
      expect(result.current.streamBlocks[0].output).toBe(
        "file1.txt\nfile2.txt",
      );
      expect(result.current.streamBlocks[0].isError).toBe(false);
    });

    it("marks tool as error when is_error is true", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-tool-end")).toBe(true);
      });

      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: {
            type: "tool_use",
            tool_use_id: "tool-err",
            tool_name: "bash",
          },
        });
      });

      act(() => {
        simulateEvent<ToolStartPayload>("agent-tool-start", {
          tool_use_id: "tool-err",
          tool_name: "bash",
          tool_input: { command: "invalid" },
          block_index: 0,
        });
      });

      act(() => {
        simulateEvent<ToolEndPayload>("agent-tool-end", {
          tool_use_id: "tool-err",
          output: "Command not found",
          is_error: true,
          block_index: 0,
        });
      });

      expect(result.current.streamBlocks[0].isError).toBe(true);
    });
  });

  describe("plan workflow", () => {
    it("sets pendingPlan on plan-ready event", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-plan-ready")).toBe(true);
      });

      act(() => {
        simulateEvent<PlanReadyPayload>("agent-plan-ready", {
          plan: "## Plan\n1. Read file\n2. Modify file",
        });
      });

      expect(result.current.pendingPlan).toBe(
        "## Plan\n1. Read file\n2. Modify file",
      );
    });

    it("approvePlan invokes correct command", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-plan-ready")).toBe(true);
      });

      act(() => {
        simulateEvent<PlanReadyPayload>("agent-plan-ready", {
          plan: "Test plan",
        });
      });

      await act(async () => {
        await result.current.approvePlan();
      });

      expect(invoke).toHaveBeenCalledWith("agent_approve_plan");
      expect(result.current.pendingPlan).toBeNull();
    });

    it("rejectPlan invokes correct command with reason", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-plan-ready")).toBe(true);
      });

      act(() => {
        simulateEvent<PlanReadyPayload>("agent-plan-ready", {
          plan: "Test plan",
        });
      });

      await act(async () => {
        await result.current.rejectPlan("Not the right approach");
      });

      expect(invoke).toHaveBeenCalledWith("agent_reject_plan", {
        reason: "Not the right approach",
      });
      expect(result.current.pendingPlan).toBeNull();
    });

    it("rejectPlan works without reason", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-plan-ready")).toBe(true);
      });

      act(() => {
        simulateEvent<PlanReadyPayload>("agent-plan-ready", {
          plan: "Test plan",
        });
      });

      await act(async () => {
        await result.current.rejectPlan();
      });

      expect(invoke).toHaveBeenCalledWith("agent_reject_plan", {
        reason: undefined,
      });
    });
  });

  describe("cancelRequest", () => {
    it("invokes agent_cancel command", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.cancelRequest();
      });

      expect(invoke).toHaveBeenCalledWith("agent_cancel");
    });

    it("sets cancelled status and stops loading", async () => {
      vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      // Start a request
      act(() => {
        result.current.sendMessage("Test");
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(true);
      });

      // Reset invoke mock to resolve for cancel
      vi.mocked(invoke).mockResolvedValue(undefined);

      await act(async () => {
        await result.current.cancelRequest();
      });

      expect(result.current.isLoading).toBe(false);
      expect(result.current.agentStatus).toBe("cancelled");
    });
  });

  describe("clearMessages", () => {
    it("clears all messages and resets state", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      // Add a message
      await act(async () => {
        await result.current.sendMessage("Test message");
      });

      expect(result.current.messages.length).toBeGreaterThan(0);

      act(() => {
        result.current.clearMessages();
      });

      expect(result.current.messages).toEqual([]);
      expect(result.current.streamBlocks).toEqual([]);
      expect(result.current.error).toBeNull();
      expect(result.current.messageQueue).toEqual([]);
    });
  });

  describe("clearError", () => {
    it("clears error state", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-error")).toBe(true);
      });

      act(() => {
        simulateEvent<AgentErrorPayload>("agent-error", {
          error: "Test error",
        });
      });

      expect(result.current.error).toBe("Test error");

      act(() => {
        result.current.clearError();
      });

      expect(result.current.error).toBeNull();
    });
  });

  describe("prompt history", () => {
    it("loads prompt history from localStorage", () => {
      localStorage.setItem(
        "devflow_prompt_history",
        JSON.stringify(["Previous prompt"]),
      );

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      expect(result.current.promptHistory).toContain("Previous prompt");
    });

    it("clearPromptHistory clears history", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.sendMessage("Test prompt");
      });

      expect(result.current.promptHistory.length).toBeGreaterThan(0);

      act(() => {
        result.current.clearPromptHistory();
      });

      expect(result.current.promptHistory).toEqual([]);
    });

    it("deduplicates prompts and keeps recent first", async () => {
      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await act(async () => {
        await result.current.sendMessage("First prompt");
      });

      await act(async () => {
        await result.current.sendMessage("Second prompt");
      });

      await act(async () => {
        await result.current.sendMessage("First prompt");
      });

      expect(result.current.promptHistory[0]).toBe("First prompt");
      expect(
        result.current.promptHistory.filter((p) => p === "First prompt").length,
      ).toBe(1);
    });
  });

  describe("event sequence - chunk to complete", () => {
    it("handles typical streaming sequence correctly", async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      const { result } = renderHook(() => useChat(), {
        wrapper: createWrapper("/test/project"),
      });

      await waitFor(() => {
        expect(eventListeners.has("agent-chunk")).toBe(true);
      });

      // User sends message
      await act(async () => {
        await result.current.sendMessage("Explain React hooks");
      });

      // Start text block
      act(() => {
        simulateEvent<ContentBlockStartPayload>("agent-content-block-start", {
          block_index: 0,
          block_type: { type: "text" },
        });
      });

      // Streaming begins
      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "React hooks ",
          block_index: 0,
        });
      });

      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "allow you ",
          block_index: 0,
        });
      });

      act(() => {
        simulateEvent<AgentChunkPayload>("agent-chunk", {
          delta: "to use state.",
          block_index: 0,
        });
      });

      expect(result.current.streamBlocks[0].text).toBe(
        "React hooks allow you to use state.",
      );

      // Complete
      act(() => {
        simulateEvent<AgentCompletePayload>("agent-complete", {
          message_id: "msg-123",
          stop_reason: "end_turn",
        });
      });

      expect(result.current.messages).toHaveLength(2);
      expect(result.current.messages[1].content_blocks).toEqual([
        { type: "text", text: "React hooks allow you to use state." },
      ]);
      expect(result.current.streamBlocks).toEqual([]);
      expect(result.current.isLoading).toBe(false);
    });
  });
});
