import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FileTree } from "./FileTree";
import type { ChangedFile } from "../types/git";

// Mock AppContext
const mockSelectFile = vi.fn();
const mockRefreshFiles = vi.fn();

interface MockAppState {
  changedFiles: ChangedFile[];
  selectedFile: string | null;
}

let mockAppState: MockAppState = {
  changedFiles: [],
  selectedFile: null,
};

vi.mock("../context/AppContext", () => ({
  useApp: () => ({
    ...mockAppState,
    selectFile: mockSelectFile,
    refreshFiles: mockRefreshFiles,
  }),
}));

// Mock CommentsContext
const mockGetCommentCountForFile = vi.fn().mockReturnValue(0);

vi.mock("../context/CommentsContext", () => ({
  useComments: () => ({
    getCommentCountForFile: mockGetCommentCountForFile,
  }),
}));

describe("FileTree", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAppState = {
      changedFiles: [],
      selectedFile: null,
    };
    mockGetCommentCountForFile.mockReturnValue(0);
  });

  describe("rendering", () => {
    it("renders header with title", () => {
      render(<FileTree />);
      expect(
        screen.getByRole("heading", { name: "Changed Files" }),
      ).toBeInTheDocument();
    });

    it("renders refresh button", () => {
      render(<FileTree />);
      expect(screen.getByTitle("Refresh files")).toBeInTheDocument();
    });

    it("renders empty state when no changed files", () => {
      mockAppState.changedFiles = [];

      render(<FileTree />);

      expect(screen.getByText("No changes detected")).toBeInTheDocument();
    });
  });

  describe("file display", () => {
    it("renders file list when files exist", () => {
      mockAppState.changedFiles = [
        {
          path: "src/main.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];

      render(<FileTree />);

      expect(screen.getByText("main.ts")).toBeInTheDocument();
      expect(screen.getByText("src/main.ts")).toBeInTheDocument();
    });

    it("renders multiple files", () => {
      mockAppState.changedFiles = [
        { path: "src/a.ts", index_status: "added", worktree_status: null },
        { path: "src/b.ts", index_status: "modified", worktree_status: null },
        { path: "src/c.ts", index_status: "deleted", worktree_status: null },
      ];

      render(<FileTree />);

      expect(screen.getByText("a.ts")).toBeInTheDocument();
      expect(screen.getByText("b.ts")).toBeInTheDocument();
      expect(screen.getByText("c.ts")).toBeInTheDocument();
    });

    it("extracts filename from path correctly", () => {
      mockAppState.changedFiles = [
        {
          path: "src/components/deep/nested/Component.tsx",
          index_status: "modified",
          worktree_status: null,
        },
      ];

      render(<FileTree />);

      expect(screen.getByText("Component.tsx")).toBeInTheDocument();
    });
  });

  describe("status icons", () => {
    it("shows + icon for added files", () => {
      mockAppState.changedFiles = [
        { path: "new.ts", index_status: "added", worktree_status: null },
      ];

      render(<FileTree />);

      expect(screen.getByText("+")).toBeInTheDocument();
    });

    it("shows ~ icon for modified files", () => {
      mockAppState.changedFiles = [
        {
          path: "modified.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];

      render(<FileTree />);

      expect(screen.getByText("~")).toBeInTheDocument();
    });

    it("shows - icon for deleted files", () => {
      mockAppState.changedFiles = [
        { path: "deleted.ts", index_status: "deleted", worktree_status: null },
      ];

      render(<FileTree />);

      expect(screen.getByText("-")).toBeInTheDocument();
    });

    it("shows → icon for renamed files", () => {
      mockAppState.changedFiles = [
        { path: "renamed.ts", index_status: "renamed", worktree_status: null },
      ];

      render(<FileTree />);

      expect(screen.getByText("→")).toBeInTheDocument();
    });

    it("shows © icon for copied files", () => {
      mockAppState.changedFiles = [
        { path: "copied.ts", index_status: "copied", worktree_status: null },
      ];

      render(<FileTree />);

      expect(screen.getByText("©")).toBeInTheDocument();
    });

    it("shows ? icon for untracked files", () => {
      mockAppState.changedFiles = [
        {
          path: "untracked.ts",
          index_status: null,
          worktree_status: "untracked",
        },
      ];

      render(<FileTree />);

      expect(screen.getByText("?")).toBeInTheDocument();
    });

    it("prefers worktree status over index status", () => {
      mockAppState.changedFiles = [
        {
          path: "file.ts",
          index_status: "added",
          worktree_status: "modified",
        },
      ];

      render(<FileTree />);

      // Should show modified (~) not added (+)
      expect(screen.getByText("~")).toBeInTheDocument();
      expect(screen.queryByText("+")).not.toBeInTheDocument();
    });
  });

  describe("file selection", () => {
    it("calls selectFile when file is clicked", async () => {
      mockAppState.changedFiles = [
        {
          path: "src/main.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];

      render(<FileTree />);

      const fileItem = screen.getByText("main.ts").closest("li");
      await userEvent.click(fileItem!);

      expect(mockSelectFile).toHaveBeenCalledWith("src/main.ts");
    });

    it("highlights selected file", () => {
      mockAppState.changedFiles = [
        { path: "src/a.ts", index_status: "modified", worktree_status: null },
        { path: "src/b.ts", index_status: "modified", worktree_status: null },
      ];
      mockAppState.selectedFile = "src/a.ts";

      render(<FileTree />);

      const fileItemA = screen.getByText("a.ts").closest("li");
      const fileItemB = screen.getByText("b.ts").closest("li");

      expect(fileItemA).toHaveClass("selected");
      expect(fileItemB).not.toHaveClass("selected");
    });

    it("sets file path as title attribute", () => {
      mockAppState.changedFiles = [
        {
          path: "src/deep/path/file.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];

      render(<FileTree />);

      const fileItem = screen.getByTitle("src/deep/path/file.ts");
      expect(fileItem).toBeInTheDocument();
    });
  });

  describe("refresh functionality", () => {
    it("calls refreshFiles when refresh button is clicked", async () => {
      render(<FileTree />);

      const refreshButton = screen.getByTitle("Refresh files");
      await userEvent.click(refreshButton);

      expect(mockRefreshFiles).toHaveBeenCalledOnce();
    });
  });

  describe("comment badges", () => {
    it("shows comment badge when file has comments", () => {
      mockAppState.changedFiles = [
        {
          path: "src/main.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];
      mockGetCommentCountForFile.mockReturnValue(3);

      render(<FileTree />);

      expect(screen.getByText("3")).toBeInTheDocument();
    });

    it("does not show comment badge when file has no comments", () => {
      mockAppState.changedFiles = [
        {
          path: "src/main.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];
      mockGetCommentCountForFile.mockReturnValue(0);

      render(<FileTree />);

      // Should not find any badge
      const badge = screen.queryByTitle(/comment/);
      expect(badge).not.toBeInTheDocument();
    });

    it("shows correct comment count", () => {
      mockAppState.changedFiles = [
        { path: "src/a.ts", index_status: "modified", worktree_status: null },
        { path: "src/b.ts", index_status: "modified", worktree_status: null },
      ];

      mockGetCommentCountForFile.mockImplementation((file: string) => {
        if (file === "src/a.ts") return 2;
        if (file === "src/b.ts") return 5;
        return 0;
      });

      render(<FileTree />);

      expect(screen.getByText("2")).toBeInTheDocument();
      expect(screen.getByText("5")).toBeInTheDocument();
    });

    it("shows singular comment tooltip for 1 comment", () => {
      mockAppState.changedFiles = [
        {
          path: "src/main.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];
      mockGetCommentCountForFile.mockReturnValue(1);

      render(<FileTree />);

      expect(screen.getByTitle("1 comment")).toBeInTheDocument();
    });

    it("shows plural comment tooltip for multiple comments", () => {
      mockAppState.changedFiles = [
        {
          path: "src/main.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];
      mockGetCommentCountForFile.mockReturnValue(5);

      render(<FileTree />);

      expect(screen.getByTitle("5 comments")).toBeInTheDocument();
    });
  });

  describe("status classes", () => {
    it("applies correct status class for added files", () => {
      mockAppState.changedFiles = [
        { path: "added.ts", index_status: "added", worktree_status: null },
      ];

      render(<FileTree />);

      const statusIcon = screen.getByText("+");
      expect(statusIcon).toHaveClass("status-added");
    });

    it("applies correct status class for modified files", () => {
      mockAppState.changedFiles = [
        {
          path: "modified.ts",
          index_status: "modified",
          worktree_status: null,
        },
      ];

      render(<FileTree />);

      const statusIcon = screen.getByText("~");
      expect(statusIcon).toHaveClass("status-modified");
    });

    it("applies correct status class for deleted files", () => {
      mockAppState.changedFiles = [
        { path: "deleted.ts", index_status: "deleted", worktree_status: null },
      ];

      render(<FileTree />);

      const statusIcon = screen.getByText("-");
      expect(statusIcon).toHaveClass("status-deleted");
    });
  });
});
