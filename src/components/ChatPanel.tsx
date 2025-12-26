import { useState, useRef, useEffect, KeyboardEvent, useMemo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import { useChat, ToolExecution } from "../context/ChatContext";
import type { ChatMessage } from "../types/agent";
import "./Panel.css";
import "./ChatPanel.css";

const TOOL_OUTPUT_TRUNCATE_LENGTH = 500;

// Extracted MarkdownContent component for reuse
interface MarkdownContentProps {
  content: string;
}

function MarkdownContent({ content }: MarkdownContentProps) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code({ className, children, ...props }) {
          const match = /language-(\w+)/.exec(className || "");
          const isInline = !match;
          return isInline ? (
            <code className="inline-code" {...props}>
              {children}
            </code>
          ) : (
            <SyntaxHighlighter
              style={oneDark}
              language={match[1]}
              PreTag="div"
              customStyle={{
                margin: 0,
                borderRadius: "4px",
                fontSize: "0.85em",
              }}
            >
              {String(children).replace(/\n$/, "")}
            </SyntaxHighlighter>
          );
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}

interface ToolBlockProps {
  execution: ToolExecution;
}

function ToolBlock({ execution }: ToolBlockProps) {
  const [inputExpanded, setInputExpanded] = useState(false);
  const [outputExpanded, setOutputExpanded] = useState(false);

  // toolInput is already an object from the backend
  const formattedInput = useMemo(() => {
    if (typeof execution.toolInput === "object") {
      return JSON.stringify(execution.toolInput, null, 2);
    }
    return String(execution.toolInput);
  }, [execution.toolInput]);

  const outputTruncated =
    execution.output && execution.output.length > TOOL_OUTPUT_TRUNCATE_LENGTH;
  const displayOutput = outputExpanded
    ? execution.output
    : execution.output?.slice(0, TOOL_OUTPUT_TRUNCATE_LENGTH);

  const getToolIcon = (name: string) => {
    switch (name) {
      case "bash":
        return "⌘";
      case "read_file":
        return "📄";
      case "write_file":
        return "✏️";
      case "edit_file":
        return "📝";
      case "list_directory":
        return "📁";
      case "web_fetch":
        return "🌐";
      case "search_web":
        return "🔍";
      case "dispatch_agent":
        return "🤖";
      case "submit_plan":
        return "📋";
      default:
        return "🔧";
    }
  };

  const getToolLabel = (name: string) => {
    switch (name) {
      case "bash":
        return "Shell Command";
      case "read_file":
        return "Read File";
      case "write_file":
        return "Write File";
      case "edit_file":
        return "Edit File";
      case "list_directory":
        return "List Directory";
      case "web_fetch":
        return "Fetch URL";
      case "search_web":
        return "Web Search";
      case "dispatch_agent":
        return "Sub-Agent";
      case "submit_plan":
        return "Submit Plan";
      default:
        return name;
    }
  };

  return (
    <div
      className={`tool-block ${execution.isComplete ? (execution.isError ? "tool-error" : "tool-success") : "tool-running"}`}
    >
      <div className="tool-header">
        <span className="tool-icon">{getToolIcon(execution.toolName)}</span>
        <span className="tool-name">{getToolLabel(execution.toolName)}</span>
        <span className="tool-status">
          {!execution.isComplete && <span className="tool-spinner" />}
          {execution.isComplete && !execution.isError && "✓"}
          {execution.isComplete && execution.isError && "✗"}
        </span>
      </div>

      <div className="tool-section">
        <button
          className="tool-toggle"
          onClick={() => setInputExpanded(!inputExpanded)}
        >
          {inputExpanded ? "▼" : "▶"} Input
        </button>
        {inputExpanded && <pre className="tool-content">{formattedInput}</pre>}
      </div>

      {execution.isComplete && execution.output && (
        <div className="tool-section">
          <button
            className="tool-toggle"
            onClick={() => setOutputExpanded(!outputExpanded)}
          >
            {outputExpanded ? "▼" : "▶"} Output
          </button>
          {(outputExpanded || !outputTruncated) && (
            <pre
              className={`tool-content ${execution.isError ? "tool-output-error" : ""}`}
            >
              {displayOutput}
              {outputTruncated && !outputExpanded && (
                <button
                  className="show-more-btn"
                  onClick={() => setOutputExpanded(true)}
                >
                  ... Show more (
                  {execution.output.length - TOOL_OUTPUT_TRUNCATE_LENGTH} more
                  chars)
                </button>
              )}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

// Component for rendering historical tool executions from saved messages
interface HistoricalToolBlockProps {
  toolExec: NonNullable<ChatMessage["tool_executions"]>[number];
}

function HistoricalToolBlock({ toolExec }: HistoricalToolBlockProps) {
  const [inputExpanded, setInputExpanded] = useState(false);
  const [outputExpanded, setOutputExpanded] = useState(false);

  const formattedInput = useMemo(() => {
    if (typeof toolExec.tool_input === "object") {
      return JSON.stringify(toolExec.tool_input, null, 2);
    }
    return String(toolExec.tool_input);
  }, [toolExec.tool_input]);

  const outputTruncated =
    toolExec.output && toolExec.output.length > TOOL_OUTPUT_TRUNCATE_LENGTH;
  const displayOutput = outputExpanded
    ? toolExec.output
    : toolExec.output?.slice(0, TOOL_OUTPUT_TRUNCATE_LENGTH);

  const getToolIcon = (name: string) => {
    switch (name) {
      case "bash":
        return "⌘";
      case "read_file":
        return "📄";
      case "write_file":
        return "✏️";
      case "edit_file":
        return "📝";
      case "list_directory":
        return "📁";
      case "web_fetch":
        return "🌐";
      case "search_web":
        return "🔍";
      case "dispatch_agent":
        return "🤖";
      case "submit_plan":
        return "📋";
      default:
        return "🔧";
    }
  };

  const getToolLabel = (name: string) => {
    switch (name) {
      case "bash":
        return "Shell Command";
      case "read_file":
        return "Read File";
      case "write_file":
        return "Write File";
      case "edit_file":
        return "Edit File";
      case "list_directory":
        return "List Directory";
      case "web_fetch":
        return "Fetch URL";
      case "search_web":
        return "Web Search";
      case "dispatch_agent":
        return "Sub-Agent";
      case "submit_plan":
        return "Submit Plan";
      default:
        return name;
    }
  };

  return (
    <div
      className={`tool-block ${toolExec.is_error ? "tool-error" : "tool-success"}`}
    >
      <div className="tool-header">
        <span className="tool-icon">{getToolIcon(toolExec.tool_name)}</span>
        <span className="tool-name">{getToolLabel(toolExec.tool_name)}</span>
        <span className="tool-status">{toolExec.is_error ? "✗" : "✓"}</span>
      </div>

      <div className="tool-section">
        <button
          className="tool-toggle"
          onClick={() => setInputExpanded(!inputExpanded)}
        >
          {inputExpanded ? "▼" : "▶"} Input
        </button>
        {inputExpanded && <pre className="tool-content">{formattedInput}</pre>}
      </div>

      {toolExec.output && (
        <div className="tool-section">
          <button
            className="tool-toggle"
            onClick={() => setOutputExpanded(!outputExpanded)}
          >
            {outputExpanded ? "▼" : "▶"} Output
          </button>
          {(outputExpanded || !outputTruncated) && (
            <pre
              className={`tool-content ${toolExec.is_error ? "tool-output-error" : ""}`}
            >
              {displayOutput}
              {outputTruncated && !outputExpanded && (
                <button
                  className="show-more-btn"
                  onClick={() => setOutputExpanded(true)}
                >
                  ... Show more (
                  {toolExec.output.length - TOOL_OUTPUT_TRUNCATE_LENGTH} more
                  chars)
                </button>
              )}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

interface PromptHistoryDropdownProps {
  history: string[];
  onSelect: (prompt: string) => void;
  onClear: () => void;
}

function PromptHistoryDropdown({
  history,
  onSelect,
  onClear,
}: PromptHistoryDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  if (history.length === 0) return null;

  return (
    <div className="prompt-history" ref={dropdownRef}>
      <button
        className="prompt-history-btn"
        onClick={() => setIsOpen(!isOpen)}
        title="Prompt history"
      >
        ↑
      </button>
      {isOpen && (
        <div className="prompt-history-dropdown">
          <div className="prompt-history-header">
            <span>Recent Prompts</span>
            <button className="prompt-history-clear" onClick={onClear}>
              Clear
            </button>
          </div>
          <div className="prompt-history-list">
            {history.map((prompt, index) => (
              <button
                key={index}
                className="prompt-history-item"
                onClick={() => {
                  onSelect(prompt);
                  setIsOpen(false);
                }}
              >
                {prompt.length > 100 ? prompt.slice(0, 100) + "..." : prompt}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

interface PlanReviewBlockProps {
  plan: string;
  onApprove: () => void;
  onReject: (reason?: string) => void;
}

function PlanReviewBlock({ plan, onApprove, onReject }: PlanReviewBlockProps) {
  const [showRejectInput, setShowRejectInput] = useState(false);
  const [rejectReason, setRejectReason] = useState("");

  // Using inline styles with CSS variables for theme consistency
  return (
    <div
      style={{
        border: "2px solid var(--color-accent)",
        borderRadius: "var(--radius-md)",
        margin: "12px 0",
        backgroundColor: "var(--color-bg-secondary)",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          padding: "10px 16px",
          backgroundColor: "var(--color-accent)",
          color: "white",
          fontWeight: 500,
          fontSize: "0.9rem",
        }}
      >
        <span>📋</span>
        <span>Plan Ready for Review</span>
      </div>

      {/* Content */}
      <div
        style={{
          padding: "16px",
          maxHeight: "300px",
          overflowY: "auto",
          color: "var(--color-text-primary)",
          fontSize: "0.875rem",
          lineHeight: 1.6,
        }}
      >
        <MarkdownContent content={plan} />
      </div>

      {/* Actions */}
      {showRejectInput ? (
        <div
          style={{
            padding: "10px 16px",
            borderTop: "1px solid var(--color-border)",
            backgroundColor: "var(--color-bg-tertiary)",
          }}
        >
          <textarea
            style={{
              width: "100%",
              padding: "8px",
              marginBottom: "8px",
              backgroundColor: "var(--color-bg-secondary)",
              border: "1px solid var(--color-border)",
              borderRadius: "4px",
              color: "var(--color-text-primary)",
              resize: "vertical",
              boxSizing: "border-box",
            }}
            placeholder="Reason for rejection (optional)"
            value={rejectReason}
            onChange={(e) => setRejectReason(e.target.value)}
            rows={2}
          />
          <div
            style={{ display: "flex", justifyContent: "flex-end", gap: "8px" }}
          >
            <button
              style={{
                padding: "8px 20px",
                borderRadius: "6px",
                fontSize: "0.85rem",
                fontWeight: 500,
                cursor: "pointer",
                backgroundColor: "transparent",
                color: "var(--color-text-muted)",
                border: "1px solid var(--color-border)",
              }}
              onClick={() => {
                setShowRejectInput(false);
                setRejectReason("");
              }}
            >
              Cancel
            </button>
            <button
              style={{
                padding: "8px 20px",
                borderRadius: "6px",
                fontSize: "0.85rem",
                fontWeight: 500,
                cursor: "pointer",
                backgroundColor: "var(--color-error)",
                color: "white",
                border: "none",
              }}
              onClick={() => onReject(rejectReason || undefined)}
            >
              Reject
            </button>
          </div>
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: "8px",
            padding: "10px 16px",
            borderTop: "1px solid var(--color-border)",
            backgroundColor: "var(--color-bg-tertiary)",
          }}
        >
          <button
            style={{
              padding: "8px 20px",
              borderRadius: "6px",
              fontSize: "0.85rem",
              fontWeight: 500,
              cursor: "pointer",
              backgroundColor: "transparent",
              color: "var(--color-error)",
              border: "1px solid var(--color-error)",
            }}
            onClick={() => setShowRejectInput(true)}
          >
            Reject
          </button>
          <button
            style={{
              padding: "8px 20px",
              borderRadius: "6px",
              fontSize: "0.85rem",
              fontWeight: 500,
              cursor: "pointer",
              backgroundColor: "var(--color-success)",
              color: "white",
              border: "none",
            }}
            onClick={onApprove}
          >
            Approve Plan
          </button>
        </div>
      )}
    </div>
  );
}

export function ChatPanel() {
  const {
    messages,
    isLoading,
    error,
    streamContent,
    toolExecutions,
    statusText,
    messageQueue,
    promptHistory,
    pendingPlan,
    sendMessage,
    cancelRequest,
    clearError,
    removeFromQueue,
    clearPromptHistory,
    approvePlan,
    rejectPlan,
  } = useChat();

  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamContent, toolExecutions]);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 150)}px`;
    }
  }, [input]);

  const handleSubmit = async () => {
    const trimmed = input.trim();
    if (!trimmed) return;
    setInput("");
    await sendMessage(trimmed);
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const handleHistorySelect = (prompt: string) => {
    setInput(prompt);
    textareaRef.current?.focus();
  };

  return (
    <div className="panel-container chat-panel">
      <div className="panel-header">
        <h2>Chat</h2>
        {statusText && (
          <div className="chat-status">
            {isLoading && <span className="status-spinner" />}
            <span className="status-text">{statusText}</span>
          </div>
        )}
      </div>

      <div className="chat-messages">
        {messages.length === 0 && !isLoading && (
          <p className="chat-empty">Start a conversation with the AI agent</p>
        )}

        {messages.map((msg) => (
          <div key={msg.id} className={`chat-message chat-message-${msg.role}`}>
            <div className="chat-message-role">
              {msg.role === "user" ? "You" : "Assistant"}
            </div>
            <div className="chat-message-content">
              {msg.role === "assistant" ? (
                <>
                  {/* Render historical tool executions */}
                  {msg.tool_executions?.map((toolExec) => (
                    <HistoricalToolBlock
                      key={toolExec.tool_use_id}
                      toolExec={toolExec}
                    />
                  ))}
                  <MarkdownContent content={msg.content} />
                </>
              ) : (
                msg.content
              )}
            </div>
          </div>
        ))}

        {/* Streaming response */}
        {isLoading && (streamContent || toolExecutions.length > 0) && (
          <div className="chat-message chat-message-assistant chat-message-streaming">
            <div className="chat-message-role">Assistant</div>
            <div className="chat-message-content">
              {/* Tool executions */}
              {toolExecutions.map((exec) => (
                <ToolBlock key={exec.toolUseId} execution={exec} />
              ))}

              {/* Streamed text */}
              {streamContent && <MarkdownContent content={streamContent} />}
              <span className="streaming-cursor" />
            </div>
          </div>
        )}

        {/* Thinking state (no content yet) */}
        {isLoading && !streamContent && toolExecutions.length === 0 && (
          <div className="chat-message chat-message-assistant">
            <div className="chat-message-role">Assistant</div>
            <div className="chat-message-content chat-typing">
              <span className="typing-dots">
                <span>.</span>
                <span>.</span>
                <span>.</span>
              </span>
            </div>
          </div>
        )}

        {/* Plan review block - shows even while loading since agent waits for approval */}
        {pendingPlan && (
          <PlanReviewBlock
            plan={pendingPlan}
            onApprove={approvePlan}
            onReject={rejectPlan}
          />
        )}

        {/* Queued messages */}
        {messageQueue.length > 0 && (
          <div className="queued-messages">
            <div className="queued-header">Queued Messages</div>
            {messageQueue.map((qm) => (
              <div key={qm.id} className="queued-message">
                <span className="queued-content">
                  {qm.content.length > 50
                    ? qm.content.slice(0, 50) + "..."
                    : qm.content}
                </span>
                <span className="queued-status">
                  {qm.status === "sending" ? "Sending..." : "Pending"}
                </span>
                {qm.status === "pending" && (
                  <button
                    className="queued-remove"
                    onClick={() => removeFromQueue(qm.id)}
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {error && (
        <div className="chat-error">
          <span>{error}</span>
          <button onClick={clearError} className="chat-error-dismiss">
            Dismiss
          </button>
        </div>
      )}

      <div className="chat-input-container">
        <PromptHistoryDropdown
          history={promptHistory}
          onSelect={handleHistorySelect}
          onClear={clearPromptHistory}
        />
        <textarea
          ref={textareaRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={
            isLoading ? "Message will be queued..." : "Type a message..."
          }
          rows={1}
          className="chat-input"
        />
        {isLoading ? (
          <button onClick={cancelRequest} className="chat-stop-btn">
            Stop
          </button>
        ) : (
          <button
            onClick={handleSubmit}
            disabled={!input.trim()}
            className="chat-send-btn"
          >
            Send
          </button>
        )}
      </div>
    </div>
  );
}
