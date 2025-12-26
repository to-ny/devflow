import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { ReactNode } from "react";
import { CommentsProvider, useComments } from "./CommentsContext";

function createWrapper() {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <CommentsProvider>{children}</CommentsProvider>;
  };
}

describe("useComments", () => {
  it("throws error when used outside CommentsProvider", () => {
    expect(() => renderHook(() => useComments())).toThrow(
      "useComments must be used within a CommentsProvider",
    );
  });
});

describe("CommentsProvider", () => {
  beforeEach(() => {
    // Clean slate for each test
  });

  describe("initial state", () => {
    it("provides empty initial state", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      expect(result.current.globalComment).toBe("");
      expect(result.current.lineComments).toEqual([]);
    });

    it("hasComments returns false when empty", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      expect(result.current.hasComments()).toBe(false);
    });
  });

  describe("global comment", () => {
    it("setGlobalComment updates global comment", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.setGlobalComment("This is a global comment");
      });

      expect(result.current.globalComment).toBe("This is a global comment");
    });

    it("hasComments returns true when global comment exists", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.setGlobalComment("Some comment");
      });

      expect(result.current.hasComments()).toBe(true);
    });

    it("hasComments returns false for whitespace-only global comment", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.setGlobalComment("   ");
      });

      expect(result.current.hasComments()).toBe(false);
    });
  });

  describe("addLineComment", () => {
    it("adds a line comment with generated id", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "src/main.ts",
          lines: { start: 10, end: 15 },
          selectedCode: "const x = 1;",
          text: "This should use let instead",
        });
      });

      expect(result.current.lineComments).toHaveLength(1);
      expect(result.current.lineComments[0].file).toBe("src/main.ts");
      expect(result.current.lineComments[0].lines.start).toBe(10);
      expect(result.current.lineComments[0].lines.end).toBe(15);
      expect(result.current.lineComments[0].selectedCode).toBe("const x = 1;");
      expect(result.current.lineComments[0].text).toBe(
        "This should use let instead",
      );
      expect(result.current.lineComments[0].id).toBeDefined();
    });

    it("adds multiple comments", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "src/a.ts",
          lines: { start: 1, end: 5 },
          selectedCode: "code a",
          text: "Comment A",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "src/b.ts",
          lines: { start: 10, end: 20 },
          selectedCode: "code b",
          text: "Comment B",
        });
      });

      expect(result.current.lineComments).toHaveLength(2);
    });

    it("generates unique ids for each comment", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment 1",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 2, end: 2 },
          selectedCode: "code",
          text: "Comment 2",
        });
      });

      const ids = result.current.lineComments.map((c) => c.id);
      expect(new Set(ids).size).toBe(2);
    });

    it("hasComments returns true when line comments exist", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      expect(result.current.hasComments()).toBe(true);
    });
  });

  describe("updateLineComment", () => {
    it("updates comment text by id", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Original text",
        });
      });

      const commentId = result.current.lineComments[0].id;

      act(() => {
        result.current.updateLineComment(commentId, "Updated text");
      });

      expect(result.current.lineComments[0].text).toBe("Updated text");
    });

    it("does not affect other comments", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "a.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code a",
          text: "Comment A",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "b.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code b",
          text: "Comment B",
        });
      });

      const firstId = result.current.lineComments[0].id;

      act(() => {
        result.current.updateLineComment(firstId, "Updated A");
      });

      expect(result.current.lineComments[0].text).toBe("Updated A");
      expect(result.current.lineComments[1].text).toBe("Comment B");
    });

    it("does nothing for non-existent id", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Original",
        });
      });

      act(() => {
        result.current.updateLineComment("non-existent-id", "New text");
      });

      expect(result.current.lineComments[0].text).toBe("Original");
    });
  });

  describe("removeLineComment", () => {
    it("removes comment by id", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      const commentId = result.current.lineComments[0].id;

      act(() => {
        result.current.removeLineComment(commentId);
      });

      expect(result.current.lineComments).toHaveLength(0);
    });

    it("only removes the specified comment", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "a.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code a",
          text: "Comment A",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "b.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code b",
          text: "Comment B",
        });
      });

      const firstId = result.current.lineComments[0].id;

      act(() => {
        result.current.removeLineComment(firstId);
      });

      expect(result.current.lineComments).toHaveLength(1);
      expect(result.current.lineComments[0].text).toBe("Comment B");
    });

    it("does nothing for non-existent id", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      act(() => {
        result.current.removeLineComment("non-existent-id");
      });

      expect(result.current.lineComments).toHaveLength(1);
    });
  });

  describe("getCommentsForFile", () => {
    it("returns comments for specified file", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "src/main.ts",
          lines: { start: 1, end: 5 },
          selectedCode: "code 1",
          text: "Comment 1",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "src/main.ts",
          lines: { start: 10, end: 15 },
          selectedCode: "code 2",
          text: "Comment 2",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "src/other.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "other code",
          text: "Other comment",
        });
      });

      const mainComments = result.current.getCommentsForFile("src/main.ts");

      expect(mainComments).toHaveLength(2);
      expect(mainComments[0].text).toBe("Comment 1");
      expect(mainComments[1].text).toBe("Comment 2");
    });

    it("returns empty array for file with no comments", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "a.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      const comments = result.current.getCommentsForFile("b.ts");

      expect(comments).toEqual([]);
    });
  });

  describe("getCommentForLine", () => {
    it("returns comment that contains the specified line", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 10, end: 20 },
          selectedCode: "selected code",
          text: "Range comment",
        });
      });

      const comment = result.current.getCommentForLine("test.ts", 15);

      expect(comment).toBeDefined();
      expect(comment!.text).toBe("Range comment");
    });

    it("returns comment for start boundary", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 10, end: 20 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      const comment = result.current.getCommentForLine("test.ts", 10);
      expect(comment).toBeDefined();
    });

    it("returns comment for end boundary", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 10, end: 20 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      const comment = result.current.getCommentForLine("test.ts", 20);
      expect(comment).toBeDefined();
    });

    it("returns undefined for line outside range", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "test.ts",
          lines: { start: 10, end: 20 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      const comment = result.current.getCommentForLine("test.ts", 5);
      expect(comment).toBeUndefined();
    });

    it("returns undefined for different file", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "a.ts",
          lines: { start: 10, end: 20 },
          selectedCode: "code",
          text: "Comment",
        });
      });

      const comment = result.current.getCommentForLine("b.ts", 15);
      expect(comment).toBeUndefined();
    });
  });

  describe("getCommentCountForFile", () => {
    it("returns correct count of comments for file", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.addLineComment({
          file: "src/main.ts",
          lines: { start: 1, end: 5 },
          selectedCode: "code",
          text: "Comment 1",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "src/main.ts",
          lines: { start: 10, end: 15 },
          selectedCode: "code",
          text: "Comment 2",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "src/other.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Other",
        });
      });

      expect(result.current.getCommentCountForFile("src/main.ts")).toBe(2);
      expect(result.current.getCommentCountForFile("src/other.ts")).toBe(1);
    });

    it("returns 0 for file with no comments", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      expect(result.current.getCommentCountForFile("nonexistent.ts")).toBe(0);
    });
  });

  describe("clearAllComments", () => {
    it("clears global comment and all line comments", () => {
      const { result } = renderHook(() => useComments(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.setGlobalComment("Global comment");
      });

      act(() => {
        result.current.addLineComment({
          file: "a.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment A",
        });
      });

      act(() => {
        result.current.addLineComment({
          file: "b.ts",
          lines: { start: 1, end: 1 },
          selectedCode: "code",
          text: "Comment B",
        });
      });

      expect(result.current.hasComments()).toBe(true);

      act(() => {
        result.current.clearAllComments();
      });

      expect(result.current.globalComment).toBe("");
      expect(result.current.lineComments).toEqual([]);
      expect(result.current.hasComments()).toBe(false);
    });
  });
});
