import {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
} from "react";

export interface LineRange {
  start: number;
  end: number;
}

export interface LineComment {
  id: string;
  file: string;
  lines: LineRange;
  selectedCode: string;
  text: string;
}

interface CommentsState {
  globalComment: string;
  lineComments: LineComment[];
}

interface CommentsContextValue extends CommentsState {
  setGlobalComment: (text: string) => void;
  addLineComment: (comment: Omit<LineComment, "id">) => void;
  updateLineComment: (id: string, text: string) => void;
  updateLineCommentWithRange: (
    id: string,
    text: string,
    lines: LineRange,
    selectedCode: string,
  ) => void;
  removeLineComment: (id: string) => void;
  getCommentsForFile: (file: string) => LineComment[];
  getCommentForLine: (file: string, lineNo: number) => LineComment | undefined;
  getOverlappingComment: (
    file: string,
    start: number,
    end: number,
  ) => LineComment | undefined;
  getCommentCountForFile: (file: string) => number;
  clearAllComments: () => void;
  hasComments: () => boolean;
}

const CommentsContext = createContext<CommentsContextValue | null>(null);

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export function CommentsProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<CommentsState>({
    globalComment: "",
    lineComments: [],
  });

  const setGlobalComment = useCallback((text: string) => {
    setState((prev) => ({ ...prev, globalComment: text }));
  }, []);

  const addLineComment = useCallback((comment: Omit<LineComment, "id">) => {
    const newComment: LineComment = {
      ...comment,
      id: generateId(),
    };
    setState((prev) => ({
      ...prev,
      lineComments: [...prev.lineComments, newComment],
    }));
  }, []);

  const updateLineComment = useCallback((id: string, text: string) => {
    setState((prev) => ({
      ...prev,
      lineComments: prev.lineComments.map((c) =>
        c.id === id ? { ...c, text } : c,
      ),
    }));
  }, []);

  const updateLineCommentWithRange = useCallback(
    (id: string, text: string, lines: LineRange, selectedCode: string) => {
      setState((prev) => ({
        ...prev,
        lineComments: prev.lineComments.map((c) =>
          c.id === id ? { ...c, text, lines, selectedCode } : c,
        ),
      }));
    },
    [],
  );

  const removeLineComment = useCallback((id: string) => {
    setState((prev) => ({
      ...prev,
      lineComments: prev.lineComments.filter((c) => c.id !== id),
    }));
  }, []);

  const getCommentsForFile = useCallback(
    (file: string) => {
      return state.lineComments.filter((c) => c.file === file);
    },
    [state.lineComments],
  );

  const getCommentForLine = useCallback(
    (file: string, lineNo: number) => {
      return state.lineComments.find(
        (c) =>
          c.file === file && lineNo >= c.lines.start && lineNo <= c.lines.end,
      );
    },
    [state.lineComments],
  );

  const getOverlappingComment = useCallback(
    (file: string, start: number, end: number) => {
      return state.lineComments.find(
        (c) =>
          c.file === file &&
          // Check if ranges overlap
          start <= c.lines.end &&
          end >= c.lines.start,
      );
    },
    [state.lineComments],
  );

  const getCommentCountForFile = useCallback(
    (file: string) => {
      return state.lineComments.filter((c) => c.file === file).length;
    },
    [state.lineComments],
  );

  const clearAllComments = useCallback(() => {
    setState({ globalComment: "", lineComments: [] });
  }, []);

  const hasComments = useCallback(() => {
    return state.globalComment.trim() !== "" || state.lineComments.length > 0;
  }, [state.globalComment, state.lineComments]);

  return (
    <CommentsContext.Provider
      value={{
        ...state,
        setGlobalComment,
        addLineComment,
        updateLineComment,
        updateLineCommentWithRange,
        removeLineComment,
        getCommentsForFile,
        getCommentForLine,
        getOverlappingComment,
        getCommentCountForFile,
        clearAllComments,
        hasComments,
      }}
    >
      {children}
    </CommentsContext.Provider>
  );
}

export function useComments() {
  const context = useContext(CommentsContext);
  if (!context) {
    throw new Error("useComments must be used within a CommentsProvider");
  }
  return context;
}
