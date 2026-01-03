import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SubagentPanel } from "./SubagentPanel";
import type {
  SubagentStartPayload,
  SubagentEndPayload,
} from "../types/generated";

// Mock Tauri event system
type EventCallback<T> = (event: { payload: T }) => void;
const eventListeners: Map<string, EventCallback<unknown>> = new Map();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, callback: EventCallback<unknown>) => {
    eventListeners.set(event, callback);
    return Promise.resolve(() => {
      eventListeners.delete(event);
    });
  }),
}));

function emitEvent<T>(event: string, payload: T) {
  const callback = eventListeners.get(event);
  if (callback) {
    act(() => {
      callback({ payload });
    });
  }
}

describe("SubagentPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventListeners.clear();
  });

  afterEach(() => {
    eventListeners.clear();
  });

  describe("visibility", () => {
    it("renders nothing when no subagents exist", () => {
      const { container } = render(<SubagentPanel />);
      expect(container.querySelector(".subagent-panel")).toBeNull();
    });

    it("shows panel when subagent starts", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Find authentication files",
      });

      await waitFor(() => {
        expect(screen.getByText(/Subagents/)).toBeInTheDocument();
      });
    });
  });

  describe("subagent display", () => {
    it("displays agent type and task", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Find authentication files",
      });

      await waitFor(() => {
        expect(screen.getByText("explore")).toBeInTheDocument();
        expect(
          screen.getByText("Find authentication files"),
        ).toBeInTheDocument();
      });
    });

    it("shows running count in header", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task 1",
      });

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-2",
        agent_type: "security-review",
        task: "Task 2",
      });

      await waitFor(() => {
        expect(screen.getByText(/2 running/)).toBeInTheDocument();
      });
    });

    it("truncates long task descriptions", async () => {
      render(<SubagentPanel />);

      const longTask =
        "This is a very long task description that should be truncated to fit within the available space";

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: longTask,
      });

      await waitFor(() => {
        expect(
          screen.getByText(/This is a very long task/),
        ).toBeInTheDocument();
        expect(screen.getByText(/\.\.\./)).toBeInTheDocument();
      });
    });
  });

  describe("status updates", () => {
    it("shows spinner for running agents", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Running task",
      });

      await waitFor(() => {
        expect(document.querySelector(".spinner")).toBeInTheDocument();
      });
    });

    it("shows checkmark for completed agents", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task",
      });

      emitEvent<SubagentEndPayload>("subagent-end", {
        id: "agent-1",
        status: "completed",
        result: "Success",
        error: null,
      });

      await waitFor(() => {
        expect(screen.getByText("✓")).toBeInTheDocument();
      });
    });

    it("shows X for failed agents", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task",
      });

      emitEvent<SubagentEndPayload>("subagent-end", {
        id: "agent-1",
        status: "failed",
        result: null,
        error: "Connection error",
      });

      await waitFor(() => {
        expect(screen.getByText("✗")).toBeInTheDocument();
        expect(screen.getByText("Connection error")).toBeInTheDocument();
      });
    });

    it("shows circle for cancelled agents", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task",
      });

      emitEvent<SubagentEndPayload>("subagent-end", {
        id: "agent-1",
        status: "cancelled",
        result: null,
        error: null,
      });

      await waitFor(() => {
        expect(screen.getByText("○")).toBeInTheDocument();
      });
    });
  });

  describe("clear functionality", () => {
    it("clear button is disabled when all agents are running", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task",
      });

      await waitFor(() => {
        const clearButton = screen.getByRole("button", { name: "Clear" });
        expect(clearButton).toBeDisabled();
      });
    });

    it("clear button is enabled when some agents are completed", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task",
      });

      emitEvent<SubagentEndPayload>("subagent-end", {
        id: "agent-1",
        status: "completed",
        result: "Done",
        error: null,
      });

      await waitFor(() => {
        const clearButton = screen.getByRole("button", { name: "Clear" });
        expect(clearButton).not.toBeDisabled();
      });
    });

    it("clicking clear removes completed agents", async () => {
      const user = userEvent.setup();
      render(<SubagentPanel />);

      // Start and complete an agent
      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Completed task",
      });

      emitEvent<SubagentEndPayload>("subagent-end", {
        id: "agent-1",
        status: "completed",
        result: "Done",
        error: null,
      });

      // Start a running agent
      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-2",
        agent_type: "plan",
        task: "Running task",
      });

      await waitFor(() => {
        expect(screen.getByText("Completed task")).toBeInTheDocument();
      });

      const clearButton = screen.getByRole("button", { name: "Clear" });
      await user.click(clearButton);

      await waitFor(() => {
        expect(screen.queryByText("Completed task")).not.toBeInTheDocument();
        expect(screen.getByText("Running task")).toBeInTheDocument();
      });
    });
  });

  describe("multiple agents", () => {
    it("handles multiple agents in parallel", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "auth",
        agent_type: "explore",
        task: "Find auth code",
      });

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "db",
        agent_type: "explore",
        task: "Find database code",
      });

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "security",
        agent_type: "security-review",
        task: "Review security",
      });

      await waitFor(() => {
        expect(screen.getByText(/3 running/)).toBeInTheDocument();
        expect(screen.getByText("Find auth code")).toBeInTheDocument();
        expect(screen.getByText("Find database code")).toBeInTheDocument();
        expect(screen.getByText("Review security")).toBeInTheDocument();
      });
    });

    it("updates correct agent on completion", async () => {
      render(<SubagentPanel />);

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-1",
        agent_type: "explore",
        task: "Task 1",
      });

      emitEvent<SubagentStartPayload>("subagent-start", {
        id: "agent-2",
        agent_type: "explore",
        task: "Task 2",
      });

      // Complete only agent-1
      emitEvent<SubagentEndPayload>("subagent-end", {
        id: "agent-1",
        status: "completed",
        result: "Done",
        error: null,
      });

      await waitFor(() => {
        expect(screen.getByText(/1 running/)).toBeInTheDocument();
        // Should have one spinner (running) and one checkmark (completed)
        expect(document.querySelectorAll(".spinner")).toHaveLength(1);
        expect(screen.getByText("✓")).toBeInTheDocument();
      });
    });
  });
});
