import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  useComments,
  LineRange,
  LineComment,
} from "../context/CommentsContext";
import { useChat } from "../context/ChatContext";
import { useNavigation } from "../context/NavigationContext";
import type { ReviewCommentsContext } from "../types/generated";
import "./CommentEditor.css";

interface CommentEditorProps {
  file: string;
  lines: LineRange;
  selectedCode: string;
  onClose: () => void;
  position: { top: number; left: number };
  existingComment?: LineComment;
}

export function CommentEditor({
  file,
  lines,
  selectedCode,
  onClose,
  position,
  existingComment,
}: CommentEditorProps) {
  const [text, setText] = useState(existingComment?.text ?? "");
  const [isSending, setIsSending] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const {
    addLineComment,
    updateLineCommentWithRange,
    removeLineComment,
    lineComments,
    globalComment,
    clearAllComments,
  } = useComments();
  const { sendMessage } = useChat();
  const { navigate } = useNavigation();
  const isEditing = !!existingComment;
  // Check if line range changed (overlapping comment with different selection)
  const rangeChanged =
    existingComment &&
    (lines.start !== existingComment.lines.start ||
      lines.end !== existingComment.lines.end);

  useEffect(() => {
    textareaRef.current?.focus({ preventScroll: true });
  }, []);

  const handleSubmit = () => {
    if (text.trim()) {
      if (isEditing) {
        // Update both text and line range (handles overlap case)
        updateLineCommentWithRange(
          existingComment.id,
          text.trim(),
          lines,
          selectedCode,
        );
      } else {
        addLineComment({ file, lines, selectedCode, text: text.trim() });
      }
      onClose();
    }
  };

  const handleDelete = () => {
    if (existingComment) {
      removeLineComment(existingComment.id);
      onClose();
    }
  };

  const handleSendAll = useCallback(async () => {
    if (!text.trim()) return;

    setIsSending(true);
    try {
      const currentComment = {
        file,
        lines: { start: lines.start, end: lines.end },
        selected_code: selectedCode,
        text: text.trim(),
      };

      let allComments;
      if (isEditing && existingComment) {
        allComments = lineComments.map((c) =>
          c.id === existingComment.id
            ? currentComment
            : {
                file: c.file,
                lines: { start: c.lines.start, end: c.lines.end },
                selected_code: c.selectedCode,
                text: c.text,
              },
        );
      } else {
        allComments = [
          ...lineComments.map((c) => ({
            file: c.file,
            lines: { start: c.lines.start, end: c.lines.end },
            selected_code: c.selectedCode,
            text: c.text,
          })),
          currentComment,
        ];
      }

      const context: ReviewCommentsContext = {
        global_comment: globalComment,
        comments: allComments,
      };

      const rendered = await invoke<string>("template_render_review_comments", {
        context,
      });

      clearAllComments();
      onClose();
      navigate("chat");
      sendMessage(rendered);
    } catch (error) {
      console.error("Failed to send comments:", error);
    } finally {
      setIsSending(false);
    }
  }, [
    text,
    file,
    lines,
    selectedCode,
    isEditing,
    existingComment,
    lineComments,
    globalComment,
    sendMessage,
    clearAllComments,
    onClose,
    navigate,
  ]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === "Escape") {
      onClose();
    }
  };

  const lineLabel =
    lines.start === lines.end
      ? `Line ${lines.start}`
      : `Lines ${lines.start}-${lines.end}`;

  const buttonLabel = isEditing
    ? rangeChanged
      ? "Update & Move"
      : "Update"
    : "Add Comment";

  return (
    <div
      className="comment-editor"
      style={{ top: position.top, left: position.left }}
    >
      <div className="comment-editor-header">
        <span className="comment-editor-label">{lineLabel}</span>
        <button className="comment-editor-close" onClick={onClose}>
          ×
        </button>
      </div>
      <textarea
        ref={textareaRef}
        className="comment-editor-input"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Add a comment... (Enter to submit, Shift+Enter for newline)"
        rows={3}
      />
      <div className="comment-editor-actions">
        {isEditing && (
          <button className="comment-editor-delete" onClick={handleDelete}>
            Delete
          </button>
        )}
        <button className="comment-editor-cancel" onClick={onClose}>
          Cancel
        </button>
        <button
          className="comment-editor-submit"
          onClick={handleSubmit}
          disabled={!text.trim() || isSending}
        >
          {buttonLabel}
        </button>
        <button
          className="comment-editor-send"
          onClick={handleSendAll}
          disabled={!text.trim() || isSending}
          title="Add this comment and send all comments to the agent"
        >
          {isSending ? "Sending..." : "Send All"}
        </button>
      </div>
    </div>
  );
}
