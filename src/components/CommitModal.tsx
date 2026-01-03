import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useChat } from "../context/ChatContext";
import { useNavigation } from "../context/NavigationContext";
import type { CommitContext } from "../types/generated";
import "./CommitModal.css";

interface CommitModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function CommitModal({ isOpen, onClose }: CommitModalProps) {
  const [instructions, setInstructions] = useState("");
  const [isSending, setIsSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { sendMessage } = useChat();
  const { navigate } = useNavigation();

  useEffect(() => {
    if (isOpen) {
      setInstructions("");
      setError(null);
      setTimeout(() => textareaRef.current?.focus(), 100);
    }
  }, [isOpen]);

  const handleSubmit = useCallback(async () => {
    setIsSending(true);
    setError(null);

    try {
      const context: CommitContext = {
        instructions: instructions.trim(),
      };

      const rendered = await invoke<string>("template_render_commit", {
        context,
      });

      onClose();
      navigate("chat");
      sendMessage(rendered);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSending(false);
    }
  }, [instructions, sendMessage, onClose, navigate]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === "Escape") {
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="commit-modal-overlay" onClick={onClose}>
      <div className="commit-modal" onClick={(e) => e.stopPropagation()}>
        <div className="commit-modal-header">
          <h3>Create Commit</h3>
          <button className="commit-modal-close" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="commit-modal-body">
          <label className="commit-modal-label">
            Commit Instructions (optional)
          </label>
          <textarea
            ref={textareaRef}
            className="commit-modal-input"
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Additional instructions for the commit... (Enter to submit, Shift+Enter for newline)"
            rows={3}
          />

          {error && <div className="commit-modal-error">{error}</div>}
        </div>

        <div className="commit-modal-actions">
          <button className="commit-modal-cancel" onClick={onClose}>
            Cancel
          </button>
          <button
            className="commit-modal-submit"
            onClick={handleSubmit}
            disabled={isSending}
          >
            {isSending ? "Sending..." : "Send to Agent"}
          </button>
        </div>
      </div>
    </div>
  );
}
