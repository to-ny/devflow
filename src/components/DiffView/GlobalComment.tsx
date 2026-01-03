import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useComments } from "../../context/CommentsContext";
import { useChat } from "../../context/ChatContext";
import { useNavigation } from "../../context/NavigationContext";
import type { ReviewCommentsContext } from "../../types/generated";

export function GlobalComment() {
  const {
    globalComment,
    setGlobalComment,
    lineComments,
    clearAllComments,
    hasComments,
  } = useComments();
  const { sendMessage } = useChat();
  const { navigate } = useNavigation();
  const [isSending, setIsSending] = useState(false);

  const handleSendComments = useCallback(async () => {
    if (!hasComments()) return;

    setIsSending(true);
    try {
      const context: ReviewCommentsContext = {
        global_comment: globalComment,
        comments: lineComments.map((c) => ({
          file: c.file,
          lines: { start: c.lines.start, end: c.lines.end },
          selected_code: c.selectedCode,
          text: c.text,
        })),
      };

      const rendered = await invoke<string>("template_render_review_comments", {
        context,
      });

      clearAllComments();
      navigate("chat");
      sendMessage(rendered);
    } catch (error) {
      console.error("Failed to send comments:", error);
    } finally {
      setIsSending(false);
    }
  }, [
    globalComment,
    lineComments,
    hasComments,
    sendMessage,
    clearAllComments,
    navigate,
  ]);

  const hasAnyComments = hasComments();

  return (
    <div className="global-comment-section">
      <label className="global-comment-label">Global Comment</label>
      <div className="global-comment-row">
        <textarea
          className="global-comment-input"
          value={globalComment}
          onChange={(e) => setGlobalComment(e.target.value)}
          placeholder="Add feedback for the entire changeset..."
          rows={2}
        />
        <button
          className="send-comments-btn"
          onClick={handleSendComments}
          disabled={isSending || !hasAnyComments}
        >
          {isSending ? "Sending..." : "Send"}
        </button>
      </div>
    </div>
  );
}
