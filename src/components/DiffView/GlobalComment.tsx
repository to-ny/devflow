import { useComments } from "../../context/CommentsContext";

export function GlobalComment() {
  const { globalComment, setGlobalComment } = useComments();

  return (
    <div className="global-comment-section">
      <label className="global-comment-label">Global Comment</label>
      <textarea
        className="global-comment-input"
        value={globalComment}
        onChange={(e) => setGlobalComment(e.target.value)}
        placeholder="Add feedback for the entire changeset..."
        rows={2}
      />
    </div>
  );
}
