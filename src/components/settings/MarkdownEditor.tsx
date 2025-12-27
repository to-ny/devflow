import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./MarkdownEditor.css";

interface MarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

type EditorMode = "edit" | "preview";

export function MarkdownEditor({
  value,
  onChange,
  placeholder,
}: MarkdownEditorProps) {
  const [mode, setMode] = useState<EditorMode>("edit");

  return (
    <div className="markdown-editor">
      <div className="markdown-editor-tabs">
        <button
          className={`markdown-editor-tab ${mode === "edit" ? "active" : ""}`}
          onClick={() => setMode("edit")}
        >
          Edit
        </button>
        <button
          className={`markdown-editor-tab ${mode === "preview" ? "active" : ""}`}
          onClick={() => setMode("preview")}
        >
          Preview
        </button>
      </div>

      {mode === "edit" ? (
        <textarea
          className="markdown-editor-textarea"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          spellCheck={false}
        />
      ) : (
        <div className="markdown-editor-preview">
          {value.trim() ? (
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{value}</ReactMarkdown>
          ) : (
            <span className="markdown-editor-empty">Nothing to preview</span>
          )}
        </div>
      )}
    </div>
  );
}
