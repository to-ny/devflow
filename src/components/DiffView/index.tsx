import { useEffect, useMemo, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../../context/AppContext";
import { useComments } from "../../context/CommentsContext";
import { CommentEditor } from "../CommentEditor";
import { FileHeader } from "./FileHeader";
import { HunkHeader, DiffLines } from "./DiffLines";
import { GlobalComment } from "./GlobalComment";
import type { CommentEditorState } from "./types";
import type { FileDiff } from "../../types/git";
import { getDisplayStatus } from "../../types/git";
import "./DiffView.css";

export function DiffView() {
  const { selectedFile, projectPath, getSelectedFileInfo } = useApp();
  const { getCommentsForFile, getCommentForLine } = useComments();
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [selectionStart, setSelectionStart] = useState<number | null>(null);
  const [selectionEnd, setSelectionEnd] = useState<number | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);
  const [commentEditor, setCommentEditor] = useState<CommentEditorState | null>(
    null,
  );
  const contentRef = useRef<HTMLDivElement>(null);
  const mousePositionRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  const commentedLines = useMemo(() => {
    if (!selectedFile) return new Set<number>();
    const comments = getCommentsForFile(selectedFile);
    const lines = new Set<number>();
    for (const comment of comments) {
      for (let i = comment.lines.start; i <= comment.lines.end; i++) {
        lines.add(i);
      }
    }
    return lines;
  }, [selectedFile, getCommentsForFile]);

  const handleLineMouseDown = useCallback(
    (lineNo: number, event: React.MouseEvent) => {
      mousePositionRef.current = { x: event.clientX, y: event.clientY };
      setSelectionStart(lineNo);
      setSelectionEnd(lineNo);
      setIsSelecting(true);
      setCommentEditor(null);
    },
    [],
  );

  const handleLineMouseEnter = useCallback(
    (lineNo: number) => {
      if (isSelecting) {
        setSelectionEnd(lineNo);
      }
    },
    [isSelecting],
  );

  const handleMouseUp = useCallback(() => {
    if (isSelecting && selectionStart !== null && diff && selectedFile) {
      setIsSelecting(false);

      const start = Math.min(selectionStart, selectionEnd ?? selectionStart);
      const end = Math.max(selectionStart, selectionEnd ?? selectionStart);

      const existingComment = getCommentForLine(selectedFile, start);
      const selectedCode =
        existingComment?.selectedCode ??
        diff.hunks
          .flatMap((h) => h.lines)
          .filter((l) => {
            const lineNo = l.new_line_no ?? l.old_line_no;
            return lineNo !== null && lineNo >= start && lineNo <= end;
          })
          .map((l) => l.content)
          .join("\n");

      const { x, y } = mousePositionRef.current;
      const editorWidth = 320;
      const editorHeight = 200;
      const padding = 10;

      const left = Math.min(
        Math.max(padding, x - editorWidth / 2),
        window.innerWidth - editorWidth - padding,
      );
      const top = Math.min(
        Math.max(padding, y + 20),
        window.innerHeight - editorHeight - padding,
      );

      setCommentEditor({
        lines: existingComment?.lines ?? { start, end },
        selectedCode,
        position: { top, left },
        existingComment,
      });
    }
  }, [
    isSelecting,
    selectionStart,
    selectionEnd,
    diff,
    selectedFile,
    getCommentForLine,
  ]);

  const handleCloseEditor = useCallback(() => {
    setCommentEditor(null);
    setSelectionStart(null);
    setSelectionEnd(null);
  }, []);

  useEffect(() => {
    if (isSelecting) {
      document.addEventListener("mouseup", handleMouseUp);
      return () => document.removeEventListener("mouseup", handleMouseUp);
    }
  }, [isSelecting, handleMouseUp]);

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

  useEffect(() => {
    setCommentEditor(null);
    setSelectionStart(null);
    setSelectionEnd(null);
  }, [selectedFile]);

  const displayStatus = useMemo(() => {
    const fileInfo = getSelectedFileInfo();
    return fileInfo ? getDisplayStatus(fileInfo) : diff?.status;
  }, [getSelectedFileInfo, diff?.status]);

  const renderContent = () => {
    if (!selectedFile) {
      return (
        <>
          <div className="diff-view-header">
            <h2>Diff</h2>
          </div>
          <div className="diff-view-empty">
            <p>Select a file to view changes</p>
          </div>
        </>
      );
    }

    if (loading) {
      return (
        <>
          <FileHeader
            filePath={selectedFile}
            projectPath={projectPath}
            status={displayStatus}
          />
          <div className="diff-view-empty">
            <p>Loading...</p>
          </div>
        </>
      );
    }

    if (error) {
      return (
        <>
          <FileHeader
            filePath={selectedFile}
            projectPath={projectPath}
            status={displayStatus}
          />
          <div className="diff-view-error">
            <p>Error: {error}</p>
          </div>
        </>
      );
    }

    if (!diff || diff.hunks.length === 0) {
      return (
        <>
          <FileHeader
            filePath={selectedFile}
            projectPath={projectPath}
            status={displayStatus}
          />
          <div className="diff-view-empty">
            <p>No changes to display</p>
          </div>
        </>
      );
    }

    return (
      <>
        <FileHeader
          filePath={selectedFile}
          projectPath={projectPath}
          status={diff.status}
        />
        <div className="diff-view-content" ref={contentRef}>
          {diff.hunks.map((hunk, index) => (
            <div key={index} className="diff-hunk">
              <HunkHeader hunk={hunk} />
              <DiffLines
                hunk={hunk}
                commentedLines={commentedLines}
                selectionStart={selectionStart}
                selectionEnd={selectionEnd}
                onLineMouseDown={handleLineMouseDown}
                onLineMouseEnter={handleLineMouseEnter}
              />
            </div>
          ))}
          {commentEditor && selectedFile && (
            <CommentEditor
              file={selectedFile}
              lines={commentEditor.lines}
              selectedCode={commentEditor.selectedCode}
              position={commentEditor.position}
              onClose={handleCloseEditor}
              existingComment={commentEditor.existingComment}
            />
          )}
        </div>
      </>
    );
  };

  return (
    <div className="diff-view">
      <GlobalComment />
      {renderContent()}
    </div>
  );
}
