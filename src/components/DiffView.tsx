import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../context/AppContext";
import type { FileDiff, DiffHunk, FileStatus } from "../types/git";
import { getDisplayStatus } from "../types/git";
import "./DiffView.css";

function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

interface FileHeaderProps {
  filePath: string;
  projectPath: string | null;
  status?: FileStatus;
}

function FileHeader({ filePath, projectPath, status }: FileHeaderProps) {
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

function HunkHeader({ hunk }: { hunk: DiffHunk }) {
  return (
    <div className="hunk-header">
      @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines}{" "}
      @@
    </div>
  );
}

function DiffLines({ hunk }: { hunk: DiffHunk }) {
  return (
    <div className="diff-lines">
      {hunk.lines.map((line, index) => (
        <div key={index} className={`diff-line ${line.kind}`}>
          <span className="line-number old">{line.old_line_no ?? ""}</span>
          <span className="line-number new">{line.new_line_no ?? ""}</span>
          <span className="line-marker">
            {line.kind === "addition"
              ? "+"
              : line.kind === "deletion"
                ? "-"
                : " "}
          </span>
          <span className="line-content">{line.content}</span>
        </div>
      ))}
    </div>
  );
}

export function DiffView() {
  const { selectedFile, projectPath, getSelectedFileInfo } = useApp();
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedFile || !projectPath) {
      setDiff(null);
      return;
    }

    let cancelled = false;

    async function fetchDiff() {
      setLoading(true);
      setError(null);

      try {
        // Get the file info to pass status (avoids redundant git status call)
        const fileInfo = getSelectedFileInfo();

        const result = await invoke<FileDiff>("git_get_file_diff_with_status", {
          projectPath,
          filePath: selectedFile,
          indexStatus: fileInfo?.index_status ?? null,
          worktreeStatus: fileInfo?.worktree_status ?? null,
        });

        if (!cancelled) {
          setDiff(result);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    fetchDiff();

    return () => {
      cancelled = true;
    };
  }, [selectedFile, projectPath, getSelectedFileInfo]);

  // Memoize display status to avoid redundant getSelectedFileInfo calls
  const displayStatus = useMemo(() => {
    const fileInfo = getSelectedFileInfo();
    return fileInfo ? getDisplayStatus(fileInfo) : diff?.status;
  }, [getSelectedFileInfo, diff?.status]);

  if (!selectedFile) {
    return (
      <div className="diff-view">
        <div className="diff-view-header">
          <h2>Diff</h2>
        </div>
        <div className="diff-view-empty">
          <p>Select a file to view changes</p>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="diff-view">
        <FileHeader
          filePath={selectedFile}
          projectPath={projectPath}
          status={displayStatus}
        />
        <div className="diff-view-empty">
          <p>Loading...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="diff-view">
        <FileHeader
          filePath={selectedFile}
          projectPath={projectPath}
          status={displayStatus}
        />
        <div className="diff-view-error">
          <p>Error: {error}</p>
        </div>
      </div>
    );
  }

  if (!diff || diff.hunks.length === 0) {
    return (
      <div className="diff-view">
        <FileHeader
          filePath={selectedFile}
          projectPath={projectPath}
          status={displayStatus}
        />
        <div className="diff-view-empty">
          <p>No changes to display</p>
        </div>
      </div>
    );
  }

  return (
    <div className="diff-view">
      <FileHeader
        filePath={selectedFile}
        projectPath={projectPath}
        status={diff.status}
      />
      <div className="diff-view-content">
        {diff.hunks.map((hunk, index) => (
          <div key={index} className="diff-hunk">
            <HunkHeader hunk={hunk} />
            <DiffLines hunk={hunk} />
          </div>
        ))}
      </div>
    </div>
  );
}
