import type { LineComment, LineRange } from "../../context/CommentsContext";

export interface CommentEditorState {
  lines: LineRange;
  selectedCode: string;
  position: { top: number; left: number };
  existingComment?: LineComment;
}
