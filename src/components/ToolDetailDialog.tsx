import { useEffect, useRef } from "react";
import { getToolIcon, getToolLabel } from "../utils/toolUtils";
import "./ToolDetailDialog.css";

export interface ToolDetail {
  toolName: string;
  toolInput: unknown;
  output?: string | null;
  isError?: boolean | null;
  isComplete: boolean;
}

interface ToolDetailDialogProps {
  tool: ToolDetail;
  onClose: () => void;
}

export function ToolDetailDialog({ tool, onClose }: ToolDetailDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  // Format the input for display
  const formattedInput =
    typeof tool.toolInput === "object"
      ? JSON.stringify(tool.toolInput, null, 2)
      : String(tool.toolInput);

  // Handle click outside to close
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        dialogRef.current &&
        !dialogRef.current.contains(event.target as Node)
      ) {
        onClose();
      }
    }

    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  // Prevent body scroll when dialog is open
  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, []);

  const statusClass = !tool.isComplete
    ? "running"
    : tool.isError
      ? "error"
      : "success";

  const statusIcon = !tool.isComplete ? "" : tool.isError ? "\u2717" : "\u2713";

  return (
    <div className="tool-dialog-overlay">
      <div className="tool-dialog" ref={dialogRef}>
        <div className={`tool-dialog-header tool-dialog-header-${statusClass}`}>
          <span className="tool-dialog-icon">{getToolIcon(tool.toolName)}</span>
          <span className="tool-dialog-title">
            {getToolLabel(tool.toolName)}
          </span>
          <span className="tool-dialog-status">
            {!tool.isComplete && <span className="tool-dialog-spinner" />}
            {statusIcon}
          </span>
          <button className="tool-dialog-close" onClick={onClose}>
            {"\u2715"}
          </button>
        </div>

        <div className="tool-dialog-body">
          <div className="tool-dialog-section">
            <div className="tool-dialog-section-header">Input</div>
            <pre className="tool-dialog-content">{formattedInput}</pre>
          </div>

          {tool.isComplete && tool.output && (
            <div className="tool-dialog-section">
              <div className="tool-dialog-section-header">
                {tool.isError ? "Error" : "Output"}
              </div>
              <pre
                className={`tool-dialog-content ${tool.isError ? "tool-dialog-content-error" : ""}`}
              >
                {tool.output}
              </pre>
            </div>
          )}

          {!tool.isComplete && (
            <div className="tool-dialog-section">
              <div className="tool-dialog-section-header">Output</div>
              <div className="tool-dialog-running">
                <span className="tool-dialog-spinner" />
                <span>Running...</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
