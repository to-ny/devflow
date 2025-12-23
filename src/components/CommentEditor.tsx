import { useState, useRef, useEffect } from "react";
import {
  useComments,
  LineRange,
  LineComment,
} from "../context/CommentsContext";
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
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { addLineComment, updateLineComment, removeLineComment } =
    useComments();
  const isEditing = !!existingComment;

  useEffect(() => {
    textareaRef.current?.focus({ preventScroll: true });
  }, []);

  const handleSubmit = () => {
    if (text.trim()) {
      if (isEditing) {
        updateLineComment(existingComment.id, text.trim());
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
          disabled={!text.trim()}
        >
          {isEditing ? "Update" : "Add Comment"}
        </button>
      </div>
    </div>
  );
}
