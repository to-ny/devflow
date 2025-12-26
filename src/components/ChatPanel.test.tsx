import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ChatPanel } from "./ChatPanel";
import type { ChatMessage } from "../types/agent";

// Mock the ChatContext
const mockSendMessage = vi.fn();
const mockCancelRequest = vi.fn();
const mockClearError = vi.fn();
const mockRemoveFromQueue = vi.fn();
const mockClearPromptHistory = vi.fn();
const mockApprovePlan = vi.fn();
const mockRejectPlan = vi.fn();

interface MockChatState {
  messages: ChatMessage[];
  isLoading: boolean;
  error: string | null;
  streamContent: string;
  toolExecutions: {
    toolUseId: string;
    toolName: string;
    toolInput: unknown;
    output?: string;
    isError?: boolean;
    isComplete: boolean;
  }[];
  agentStatus: string;
  statusText: string;
  messageQueue: { id: string; content: string; status: string }[];
  promptHistory: string[];
  pendingPlan: string | null;
}

let mockChatState: MockChatState = {
  messages: [],
  isLoading: false,
  error: null,
  streamContent: "",
  toolExecutions: [],
  agentStatus: "idle",
  statusText: "",
  messageQueue: [],
  promptHistory: [],
  pendingPlan: null,
};

vi.mock("../context/ChatContext", () => ({
  useChat: () => ({
    ...mockChatState,
    sendMessage: mockSendMessage,
    cancelRequest: mockCancelRequest,
    clearError: mockClearError,
    removeFromQueue: mockRemoveFromQueue,
    clearPromptHistory: mockClearPromptHistory,
    approvePlan: mockApprovePlan,
    rejectPlan: mockRejectPlan,
  }),
}));

describe("ChatPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockChatState = {
      messages: [],
      isLoading: false,
      error: null,
      streamContent: "",
      toolExecutions: [],
      agentStatus: "idle",
      statusText: "",
      messageQueue: [],
      promptHistory: [],
      pendingPlan: null,
    };
  });

  describe("rendering", () => {
    it("renders chat header", () => {
      render(<ChatPanel />);
      expect(screen.getByRole("heading", { name: "Chat" })).toBeInTheDocument();
    });

    it("renders empty state message when no messages", () => {
      render(<ChatPanel />);
      expect(
        screen.getByText("Start a conversation with the AI agent"),
      ).toBeInTheDocument();
    });

    it("renders input textarea", () => {
      render(<ChatPanel />);
      expect(
        screen.getByPlaceholderText("Type a message..."),
      ).toBeInTheDocument();
    });

    it("renders send button", () => {
      render(<ChatPanel />);
      expect(screen.getByRole("button", { name: "Send" })).toBeInTheDocument();
    });
  });

  describe("messages display", () => {
    it("renders user messages", () => {
      mockChatState.messages = [
        { id: "msg-1", role: "user", content: "Hello, AI!" },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("You")).toBeInTheDocument();
      expect(screen.getByText("Hello, AI!")).toBeInTheDocument();
    });

    it("renders assistant messages", () => {
      mockChatState.messages = [
        { id: "msg-1", role: "assistant", content: "Hello! How can I help?" },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Assistant")).toBeInTheDocument();
      expect(screen.getByText("Hello! How can I help?")).toBeInTheDocument();
    });

    it("renders multiple messages in order", () => {
      mockChatState.messages = [
        { id: "msg-1", role: "user", content: "First message" },
        { id: "msg-2", role: "assistant", content: "First response" },
        { id: "msg-3", role: "user", content: "Second message" },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("First message")).toBeInTheDocument();
      expect(screen.getByText("First response")).toBeInTheDocument();
      expect(screen.getByText("Second message")).toBeInTheDocument();
    });

    it("renders markdown in assistant messages", () => {
      mockChatState.messages = [
        {
          id: "msg-1",
          role: "assistant",
          content: "Here is some **bold** text",
        },
      ];

      render(<ChatPanel />);

      // Check that bold text is rendered
      const boldElement = screen.getByText("bold");
      expect(boldElement.tagName).toBe("STRONG");
    });
  });

  describe("tool blocks", () => {
    it("renders tool execution blocks in streaming state", () => {
      mockChatState.isLoading = true;
      mockChatState.toolExecutions = [
        {
          toolUseId: "tool-1",
          toolName: "read_file",
          toolInput: { path: "/test/file.ts" },
          isComplete: false,
        },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Read File")).toBeInTheDocument();
    });

    it("shows tool running state", () => {
      mockChatState.isLoading = true;
      mockChatState.toolExecutions = [
        {
          toolUseId: "tool-1",
          toolName: "bash",
          toolInput: { command: "ls" },
          isComplete: false,
        },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Shell Command")).toBeInTheDocument();
    });

    it("shows tool success state", () => {
      mockChatState.isLoading = true;
      mockChatState.toolExecutions = [
        {
          toolUseId: "tool-1",
          toolName: "read_file",
          toolInput: { path: "/test/file.ts" },
          output: "file contents",
          isError: false,
          isComplete: true,
        },
      ];

      render(<ChatPanel />);

      // Check for success indicator
      expect(screen.getByText("✓")).toBeInTheDocument();
    });

    it("shows tool error state", () => {
      mockChatState.isLoading = true;
      mockChatState.toolExecutions = [
        {
          toolUseId: "tool-1",
          toolName: "bash",
          toolInput: { command: "invalid" },
          output: "Error: command not found",
          isError: true,
          isComplete: true,
        },
      ];

      render(<ChatPanel />);

      // Check for error indicator
      expect(screen.getByText("✗")).toBeInTheDocument();
    });

    it("renders historical tool executions in messages", () => {
      mockChatState.messages = [
        {
          id: "msg-1",
          role: "assistant",
          content: "I read the file.",
          tool_executions: [
            {
              tool_use_id: "tool-1",
              tool_name: "read_file",
              tool_input: { path: "/test.ts" },
              output: "content",
              is_error: false,
            },
          ],
        },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Read File")).toBeInTheDocument();
    });
  });

  describe("plan review", () => {
    it("displays plan review block when pendingPlan exists", () => {
      mockChatState.pendingPlan = "## Plan\n1. Step one\n2. Step two";

      render(<ChatPanel />);

      expect(screen.getByText("Plan Ready for Review")).toBeInTheDocument();
      expect(screen.getByText("Step one")).toBeInTheDocument();
    });

    it("calls approvePlan when approve button is clicked", async () => {
      mockChatState.pendingPlan = "Test plan";

      render(<ChatPanel />);

      const approveButton = screen.getByRole("button", {
        name: "Approve Plan",
      });
      await userEvent.click(approveButton);

      expect(mockApprovePlan).toHaveBeenCalledOnce();
    });

    it("shows reject input when reject button is clicked", async () => {
      mockChatState.pendingPlan = "Test plan";

      render(<ChatPanel />);

      const rejectButton = screen.getByRole("button", { name: "Reject" });
      await userEvent.click(rejectButton);

      expect(
        screen.getByPlaceholderText("Reason for rejection (optional)"),
      ).toBeInTheDocument();
    });

    it("calls rejectPlan with reason", async () => {
      mockChatState.pendingPlan = "Test plan";

      render(<ChatPanel />);

      // Click reject to show input
      const rejectButton = screen.getByRole("button", { name: "Reject" });
      await userEvent.click(rejectButton);

      // Enter reason
      const textarea = screen.getByPlaceholderText(
        "Reason for rejection (optional)",
      );
      await userEvent.type(textarea, "Not the right approach");

      // Click final reject button
      const finalRejectButton = screen.getAllByRole("button", {
        name: "Reject",
      })[0];
      await userEvent.click(finalRejectButton);

      expect(mockRejectPlan).toHaveBeenCalledWith("Not the right approach");
    });
  });

  describe("loading state", () => {
    it("shows thinking dots when loading with no content", () => {
      mockChatState.isLoading = true;
      mockChatState.streamContent = "";
      mockChatState.toolExecutions = [];

      render(<ChatPanel />);

      // Check for typing dots
      const dots = screen.getAllByText(".");
      expect(dots.length).toBeGreaterThan(0);
    });

    it("shows streaming content when available", () => {
      mockChatState.isLoading = true;
      mockChatState.streamContent = "This is streaming text";

      render(<ChatPanel />);

      expect(screen.getByText("This is streaming text")).toBeInTheDocument();
    });

    it("shows status text when loading", () => {
      mockChatState.isLoading = true;
      mockChatState.statusText = "Analyzing code...";

      render(<ChatPanel />);

      expect(screen.getByText("Analyzing code...")).toBeInTheDocument();
    });

    it("shows stop button instead of send when loading", () => {
      mockChatState.isLoading = true;

      render(<ChatPanel />);

      expect(screen.getByRole("button", { name: "Stop" })).toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: "Send" }),
      ).not.toBeInTheDocument();
    });

    it("shows queuing placeholder when loading", () => {
      mockChatState.isLoading = true;

      render(<ChatPanel />);

      expect(
        screen.getByPlaceholderText("Message will be queued..."),
      ).toBeInTheDocument();
    });
  });

  describe("error handling", () => {
    it("displays error message when error exists", () => {
      mockChatState.error = "API connection failed";

      render(<ChatPanel />);

      expect(screen.getByText("API connection failed")).toBeInTheDocument();
    });

    it("calls clearError when dismiss button is clicked", async () => {
      mockChatState.error = "Some error";

      render(<ChatPanel />);

      const dismissButton = screen.getByRole("button", { name: "Dismiss" });
      await userEvent.click(dismissButton);

      expect(mockClearError).toHaveBeenCalledOnce();
    });
  });

  describe("message queue", () => {
    it("displays queued messages", () => {
      mockChatState.messageQueue = [
        { id: "q-1", content: "Queued message one", status: "pending" },
        { id: "q-2", content: "Queued message two", status: "pending" },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Queued Messages")).toBeInTheDocument();
      expect(screen.getByText(/Queued message one/)).toBeInTheDocument();
    });

    it("shows pending status for queued messages", () => {
      mockChatState.messageQueue = [
        { id: "q-1", content: "Pending message", status: "pending" },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Pending")).toBeInTheDocument();
    });

    it("shows sending status for messages being sent", () => {
      mockChatState.messageQueue = [
        { id: "q-1", content: "Sending message", status: "sending" },
      ];

      render(<ChatPanel />);

      expect(screen.getByText("Sending...")).toBeInTheDocument();
    });

    it("calls removeFromQueue when remove button clicked", async () => {
      mockChatState.messageQueue = [
        { id: "q-1", content: "Pending message", status: "pending" },
      ];

      render(<ChatPanel />);

      const removeButton = screen.getByText("✕");
      await userEvent.click(removeButton);

      expect(mockRemoveFromQueue).toHaveBeenCalledWith("q-1");
    });
  });

  describe("sending messages", () => {
    it("calls sendMessage when send button is clicked", async () => {
      render(<ChatPanel />);

      const textarea = screen.getByPlaceholderText("Type a message...");
      await userEvent.type(textarea, "Test message");

      const sendButton = screen.getByRole("button", { name: "Send" });
      await userEvent.click(sendButton);

      expect(mockSendMessage).toHaveBeenCalledWith("Test message");
    });

    it("calls sendMessage when Enter is pressed", async () => {
      render(<ChatPanel />);

      const textarea = screen.getByPlaceholderText("Type a message...");
      await userEvent.type(textarea, "Test message{Enter}");

      expect(mockSendMessage).toHaveBeenCalledWith("Test message");
    });

    it("does not send on Shift+Enter", async () => {
      render(<ChatPanel />);

      const textarea = screen.getByPlaceholderText("Type a message...");

      // Type message and press Shift+Enter
      fireEvent.change(textarea, { target: { value: "Test message" } });
      fireEvent.keyDown(textarea, { key: "Enter", shiftKey: true });

      expect(mockSendMessage).not.toHaveBeenCalled();
    });

    it("clears input after sending", async () => {
      render(<ChatPanel />);

      const textarea = screen.getByPlaceholderText(
        "Type a message...",
      ) as HTMLTextAreaElement;
      await userEvent.type(textarea, "Test message");

      const sendButton = screen.getByRole("button", { name: "Send" });
      await userEvent.click(sendButton);

      expect(textarea.value).toBe("");
    });

    it("disables send button when input is empty", () => {
      render(<ChatPanel />);

      const sendButton = screen.getByRole("button", { name: "Send" });
      expect(sendButton).toBeDisabled();
    });
  });

  describe("cancel request", () => {
    it("calls cancelRequest when stop button is clicked", async () => {
      mockChatState.isLoading = true;

      render(<ChatPanel />);

      const stopButton = screen.getByRole("button", { name: "Stop" });
      await userEvent.click(stopButton);

      expect(mockCancelRequest).toHaveBeenCalledOnce();
    });
  });

  describe("prompt history", () => {
    it("shows history button when history exists", () => {
      mockChatState.promptHistory = ["Previous prompt"];

      render(<ChatPanel />);

      expect(screen.getByTitle("Prompt history")).toBeInTheDocument();
    });

    it("does not show history button when history is empty", () => {
      mockChatState.promptHistory = [];

      render(<ChatPanel />);

      expect(screen.queryByTitle("Prompt history")).not.toBeInTheDocument();
    });

    it("opens history dropdown when button is clicked", async () => {
      mockChatState.promptHistory = ["Prompt 1", "Prompt 2"];

      render(<ChatPanel />);

      const historyButton = screen.getByTitle("Prompt history");
      await userEvent.click(historyButton);

      expect(screen.getByText("Recent Prompts")).toBeInTheDocument();
      expect(screen.getByText("Prompt 1")).toBeInTheDocument();
      expect(screen.getByText("Prompt 2")).toBeInTheDocument();
    });

    it("populates input when history item is selected", async () => {
      mockChatState.promptHistory = ["Selected prompt"];

      render(<ChatPanel />);

      const historyButton = screen.getByTitle("Prompt history");
      await userEvent.click(historyButton);

      const historyItem = screen.getByText("Selected prompt");
      await userEvent.click(historyItem);

      const textarea = screen.getByPlaceholderText(
        "Type a message...",
      ) as HTMLTextAreaElement;
      expect(textarea.value).toBe("Selected prompt");
    });

    it("calls clearPromptHistory when clear button is clicked", async () => {
      mockChatState.promptHistory = ["Prompt"];

      render(<ChatPanel />);

      const historyButton = screen.getByTitle("Prompt history");
      await userEvent.click(historyButton);

      const clearButton = screen.getByRole("button", { name: "Clear" });
      await userEvent.click(clearButton);

      expect(mockClearPromptHistory).toHaveBeenCalledOnce();
    });
  });
});
