import { describe, it, expect } from "vitest";

import type { ProjectConfig } from "../types/config";
import type { ChangedFile } from "../types/git";
import type { AgentCompletePayload, ChatMessage } from "../types/agent";

/**
 * Contract tests validate TypeScript types match Rust-generated types.
 * The primary value is compile-time checking - if types drift, tsc fails.
 * Runtime tests here verify serialization compatibility.
 */
describe("Contract Tests", () => {
  it("ProjectConfig serializes correctly", () => {
    const config: ProjectConfig = {
      agent: {
        provider: "anthropic",
        model: "claude-sonnet-4-20250514",
        api_key_env: "ANTHROPIC_API_KEY",
        max_tokens: 8192,
      },
      prompts: { pre: "", post: "" },
      execution: {
        timeout_secs: 30,
        max_tool_iterations: 50,
        max_agent_depth: 3,
      },
      notifications: { on_complete: [], on_error: [] },
      search: { provider: "duckduckgo", max_results: 10 },
    };

    const roundTripped: ProjectConfig = JSON.parse(JSON.stringify(config));
    expect(roundTripped.agent.provider).toBe("anthropic");
    expect(roundTripped.execution.max_agent_depth).toBe(3);
  });

  it("ChangedFile handles null status values", () => {
    const files: ChangedFile[] = [
      { path: "a.ts", index_status: "added", worktree_status: null },
      { path: "b.ts", index_status: null, worktree_status: "modified" },
    ];

    const roundTripped: ChangedFile[] = JSON.parse(JSON.stringify(files));
    expect(roundTripped[0].worktree_status).toBeNull();
    expect(roundTripped[1].index_status).toBeNull();
  });

  it("AgentCompletePayload handles null stop_reason", () => {
    const payload: AgentCompletePayload = {
      message_id: "msg_123",
      stop_reason: null,
    };

    const roundTripped: AgentCompletePayload = JSON.parse(
      JSON.stringify(payload),
    );
    expect(roundTripped.stop_reason).toBeNull();
  });

  it("ChatMessage with content_blocks serializes correctly", () => {
    const message: ChatMessage = {
      id: "msg-1",
      role: "assistant",
      content_blocks: [
        { type: "text", text: "Response" },
        {
          type: "tool_use",
          tool_use_id: "tool-1",
          tool_name: "read_file",
          tool_input: { path: "/test.ts" },
          output: "content",
          is_error: null,
        },
      ],
    };

    const roundTripped: ChatMessage = JSON.parse(JSON.stringify(message));
    expect(roundTripped.content_blocks).toHaveLength(2);
    expect(roundTripped.content_blocks[0].type).toBe("text");
    expect(roundTripped.content_blocks[1].type).toBe("tool_use");
  });
});
