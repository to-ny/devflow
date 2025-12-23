import { useApp } from "../context/AppContext";
import { useComments } from "../context/CommentsContext";
import type { FileStatus } from "../types/git";
import { getDisplayStatus } from "../types/git";
import "./FileTree.css";

const STATUS_ICONS: Record<FileStatus, string> = {
  added: "+",
  modified: "~",
  deleted: "-",
  renamed: "→",
  copied: "©",
  untracked: "?",
};

const STATUS_CLASSES: Record<FileStatus, string> = {
  added: "status-added",
  modified: "status-modified",
  deleted: "status-deleted",
  renamed: "status-renamed",
  copied: "status-copied",
  untracked: "status-untracked",
};

function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

export function FileTree() {
  const { changedFiles, selectedFile, selectFile, refreshFiles } = useApp();
  const { getCommentCountForFile } = useComments();

  return (
    <div className="file-tree">
      <div className="file-tree-header">
        <h2>Changed Files</h2>
        <button
          className="refresh-btn"
          onClick={refreshFiles}
          title="Refresh files"
        >
          ↻
        </button>
      </div>
      <div className="file-tree-content">
        {changedFiles.length === 0 ? (
          <p className="empty-message">No changes detected</p>
        ) : (
          <ul className="file-list">
            {changedFiles.map((file) => {
              const status = getDisplayStatus(file);
              const commentCount = getCommentCountForFile(file.path);
              return (
                <li
                  key={file.path}
                  className={`file-item ${selectedFile === file.path ? "selected" : ""}`}
                  onClick={() => selectFile(file.path)}
                  title={file.path}
                >
                  <span className={`file-status ${STATUS_CLASSES[status]}`}>
                    {STATUS_ICONS[status]}
                  </span>
                  <span className="file-name">{getFileName(file.path)}</span>
                  <span className="file-path">{file.path}</span>
                  {commentCount > 0 && (
                    <span
                      className="comment-badge"
                      title={`${commentCount} comment${commentCount > 1 ? "s" : ""}`}
                    >
                      {commentCount}
                    </span>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
