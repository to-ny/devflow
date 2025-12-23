import { useState, useRef, useEffect, KeyboardEvent } from "react";
import { useChat } from "../context/ChatContext";
import "./Panel.css";
import "./ChatPanel.css";

export function ChatPanel() {
  const { messages, isLoading, error, streamContent, sendMessage, clearError } =
    useChat();
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamContent]);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 150)}px`;
    }
  }, [input]);

  const handleSubmit = async () => {
    const trimmed = input.trim();
    if (!trimmed || isLoading) return;
    setInput("");
    await sendMessage(trimmed);
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="panel-container chat-panel">
      <div className="panel-header">
        <h2>Chat</h2>
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
            <div className="chat-message-content">{msg.content}</div>
          </div>
        ))}

        {isLoading && streamContent && (
          <div className="chat-message chat-message-assistant">
            <div className="chat-message-role">Assistant</div>
            <div className="chat-message-content">{streamContent}</div>
          </div>
        )}

        {isLoading && !streamContent && (
          <div className="chat-message chat-message-assistant">
            <div className="chat-message-role">Assistant</div>
            <div className="chat-message-content chat-typing">Thinking...</div>
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
        <textarea
          ref={textareaRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message..."
          disabled={isLoading}
          rows={1}
          className="chat-input"
        />
        <button
          onClick={handleSubmit}
          disabled={!input.trim() || isLoading}
          className="chat-send-btn"
        >
          Send
        </button>
      </div>
    </div>
  );
}
