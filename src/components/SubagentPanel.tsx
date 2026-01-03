import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  SubagentStartPayload,
  SubagentEndPayload,
  SubagentStatus,
} from "../types/generated";
import "./SubagentPanel.css";

interface ActiveSubagent {
  id: string;
  agentType: string;
  task: string;
  status: SubagentStatus | "running";
  startTime: number;
  endTime?: number;
  error?: string;
}

export function SubagentPanel() {
  const [subagents, setSubagents] = useState<ActiveSubagent[]>([]);

  useEffect(() => {
    const unlistenStart = listen<SubagentStartPayload>(
      "subagent-start",
      (event) => {
        setSubagents((prev) => [
          ...prev,
          {
            id: event.payload.id,
            agentType: event.payload.agent_type,
            task: event.payload.task,
            status: "running",
            startTime: Date.now(),
          },
        ]);
      },
    );

    const unlistenEnd = listen<SubagentEndPayload>("subagent-end", (event) => {
      setSubagents((prev) =>
        prev.map((agent) =>
          agent.id === event.payload.id
            ? {
                ...agent,
                status: event.payload.status,
                endTime: Date.now(),
                error: event.payload.error ?? undefined,
              }
            : agent,
        ),
      );
    });

    return () => {
      unlistenStart.then((fn) => fn());
      unlistenEnd.then((fn) => fn());
    };
  }, []);

  const clearCompleted = useCallback(() => {
    setSubagents((prev) => prev.filter((agent) => agent.status === "running"));
  }, []);

  const activeCount = subagents.filter((a) => a.status === "running").length;

  if (subagents.length === 0) {
    return null;
  }

  return (
    <div className="subagent-panel">
      <div className="subagent-panel-header">
        <span className="subagent-panel-title">
          Subagents {activeCount > 0 && `(${activeCount} running)`}
        </span>
        <button
          className="subagent-panel-clear"
          onClick={clearCompleted}
          disabled={activeCount === subagents.length}
        >
          Clear
        </button>
      </div>
      <div className="subagent-panel-list">
        {subagents.map((agent) => (
          <SubagentItem key={agent.id} agent={agent} />
        ))}
      </div>
    </div>
  );
}

function SubagentItem({ agent }: { agent: ActiveSubagent }) {
  const duration =
    agent.endTime && agent.startTime
      ? ((agent.endTime - agent.startTime) / 1000).toFixed(1)
      : null;

  return (
    <div className={`subagent-item subagent-item--${agent.status}`}>
      <div className="subagent-item-header">
        <span className="subagent-item-type">{agent.agentType}</span>
        <span className="subagent-item-status">
          {agent.status === "running" && <span className="spinner" />}
          {agent.status === "completed" && "✓"}
          {agent.status === "failed" && "✗"}
          {agent.status === "cancelled" && "○"}
        </span>
      </div>
      <div className="subagent-item-task">{truncateTask(agent.task)}</div>
      {duration && <div className="subagent-item-duration">{duration}s</div>}
      {agent.error && <div className="subagent-item-error">{agent.error}</div>}
    </div>
  );
}

function truncateTask(task: string): string {
  if (task.length <= 60) return task;
  return task.slice(0, 57) + "...";
}
