import type { FileStatus } from "../../types/git";

function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

interface FileHeaderProps {
  filePath: string;
  projectPath: string | null;
  status?: FileStatus;
}

export function FileHeader({ filePath, projectPath, status }: FileHeaderProps) {
  const fileName = getFileName(filePath);
  const fullPath = projectPath ? `${projectPath}/${filePath}` : filePath;

  return (
    <div className="diff-view-header">
      <div className="file-info">
        <h2>{fileName}</h2>
        <span className="file-full-path" title={fullPath}>
          {filePath}
        </span>
      </div>
      {status && (
        <span className={`file-status-badge ${status}`}>{status}</span>
      )}
    </div>
  );
}
