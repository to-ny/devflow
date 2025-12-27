import { useState, useRef, useEffect, KeyboardEvent } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import { useChat, ToolExecution } from "../context/ChatContext";
import type { ChatMessage } from "../types/agent";
import { getToolIcon, getToolLabel } from "../utils/toolUtils";
import { ToolDetailDialog, ToolDetail } from "./ToolDetailDialog";
import "./Panel.css";
import "./ChatPanel.css";

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
  onOpenDetail: (tool: ToolDetail) => void;
}

function ToolBlock({ execution, onOpenDetail }: ToolBlockProps) {
  const handleClick = () => {
    onOpenDetail({
      toolName: execution.toolName,
      toolInput: execution.toolInput,
      output: execution.output,
      isError: execution.isError,
      isComplete: execution.isComplete,
    });
  };

  const statusClass = execution.isComplete
    ? execution.isError
      ? "tool-error"
      : "tool-success"
    : "tool-running";

  return (
    <div className={`tool-row ${statusClass}`} onClick={handleClick}>
      <span className="tool-row-icon">{getToolIcon(execution.toolName)}</span>
      <span className="tool-row-name">{getToolLabel(execution.toolName)}</span>
      <span className="tool-row-status">
        {!execution.isComplete && <span className="tool-spinner" />}
        {execution.isComplete && !execution.isError && "\u2713"}
        {execution.isComplete && execution.isError && "\u2717"}
      </span>
    </div>
  );
}

// Component for rendering historical tool executions from saved messages
interface HistoricalToolBlockProps {
  toolExec: NonNullable<ChatMessage["tool_executions"]>[number];
  onOpenDetail: (tool: ToolDetail) => void;
}

function HistoricalToolBlock({
  toolExec,
  onOpenDetail,
}: HistoricalToolBlockProps) {
  const handleClick = () => {
    onOpenDetail({
      toolName: toolExec.tool_name,
      toolInput: toolExec.tool_input,
      output: toolExec.output,
      isError: toolExec.is_error,
      isComplete: true, // Historical executions are always complete
    });
  };

  const statusClass = toolExec.is_error ? "tool-error" : "tool-success";

  return (
    <div className={`tool-row ${statusClass}`} onClick={handleClick}>
      <span className="tool-row-icon">{getToolIcon(toolExec.tool_name)}</span>
      <span className="tool-row-name">{getToolLabel(toolExec.tool_name)}</span>
      <span className="tool-row-status">
        {toolExec.is_error ? "\u2717" : "\u2713"}
      </span>
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
    clearMessages,
    clearError,
    removeFromQueue,
    clearPromptHistory,
    approvePlan,
    rejectPlan,
  } = useChat();

  const [input, setInput] = useState("");
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const [selectedTool, setSelectedTool] = useState<ToolDetail | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleClearClick = () => {
    setShowClearConfirm(true);
  };

  const handleClearConfirm = () => {
    clearMessages();
    setShowClearConfirm(false);
  };

  const handleClearCancel = () => {
    setShowClearConfirm(false);
  };

  const handleOpenToolDetail = (tool: ToolDetail) => {
    setSelectedTool(tool);
  };

  const handleCloseToolDetail = () => {
    setSelectedTool(null);
  };

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
        <div className="panel-header-actions">
          {statusText && (
            <div className="chat-status">
              {isLoading && <span className="status-spinner" />}
              <span className="status-text">{statusText}</span>
            </div>
          )}
          <button
            className="chat-clear-btn"
            onClick={handleClearClick}
            disabled={isLoading || messages.length === 0}
            title="Clear chat"
          >
            Clear
          </button>
        </div>
      </div>

      {/* Clear confirmation dialog */}
      {showClearConfirm && (
        <div className="confirm-dialog-overlay">
          <div className="confirm-dialog">
            <div className="confirm-dialog-header">Clear Chat</div>
            <div className="confirm-dialog-body">
              Are you sure you want to clear all messages? This action cannot be
              undone.
            </div>
            <div className="confirm-dialog-actions">
              <button
                className="confirm-dialog-cancel"
                onClick={handleClearCancel}
              >
                Cancel
              </button>
              <button
                className="confirm-dialog-confirm"
                onClick={handleClearConfirm}
              >
                Clear
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Tool detail dialog */}
      {selectedTool && (
        <ToolDetailDialog tool={selectedTool} onClose={handleCloseToolDetail} />
      )}

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
                      onOpenDetail={handleOpenToolDetail}
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
                <ToolBlock
                  key={exec.toolUseId}
                  execution={exec}
                  onOpenDetail={handleOpenToolDetail}
                />
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
