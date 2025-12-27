import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./MarkdownPreview.css";

interface MarkdownPreviewProps {
  content: string;
  maxHeight?: number;
  truncate?: boolean;
}

export function MarkdownPreview({
  content,
  maxHeight = 200,
  truncate = true,
}: MarkdownPreviewProps) {
  if (!content || !content.trim()) {
    return <div className="markdown-preview-empty">No content configured</div>;
  }

  const displayContent =
    truncate && content.length > 500
      ? content.substring(0, 500) + "..."
      : content;

  return (
    <div
      className="markdown-preview"
      style={{ maxHeight: truncate ? maxHeight : undefined }}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>
        {displayContent}
      </ReactMarkdown>
      {truncate && content.length > 500 && (
        <div className="markdown-preview-fade" />
      )}
    </div>
  );
}
