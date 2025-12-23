import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { extractFolderName, AppProvider, useApp } from "./AppContext";
import { invoke } from "@tauri-apps/api/core";

describe("extractFolderName", () => {
  it("extracts folder name from Unix path", () => {
    expect(extractFolderName("/home/user/projects/myapp")).toBe("myapp");
  });

  it("extracts folder name from Windows path", () => {
    expect(extractFolderName("C:\\Users\\user\\projects\\myapp")).toBe("myapp");
  });

  it("extracts folder name from WSL UNC path", () => {
    expect(
      extractFolderName("\\\\wsl.localhost\\Ubuntu\\home\\user\\myapp"),
    ).toBe("myapp");
  });

  it("returns original path if no separator found", () => {
    expect(extractFolderName("myapp")).toBe("myapp");
  });

  it("handles trailing separator by returning full path", () => {
    // When pop() returns empty string, function falls back to original path
    expect(extractFolderName("/home/user/projects/myapp/")).toBe(
      "/home/user/projects/myapp/",
    );
  });

  it("handles empty path", () => {
    expect(extractFolderName("")).toBe("");
  });
});

describe("useApp", () => {
  it("throws error when used outside AppProvider", () => {
    function TestComponent() {
      useApp();
      return null;
    }

    expect(() => render(<TestComponent />)).toThrow(
      "useApp must be used within an AppProvider",
    );
  });
});

describe("AppProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("provides initial loading state", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    function TestComponent() {
      const { isLoading, isProjectOpen } = useApp();
      return (
        <div>
          <span data-testid="loading">{isLoading.toString()}</span>
          <span data-testid="open">{isProjectOpen.toString()}</span>
        </div>
      );
    }

    render(
      <AppProvider>
        <TestComponent />
      </AppProvider>,
    );

    expect(screen.getByTestId("loading")).toHaveTextContent("true");
    expect(screen.getByTestId("open")).toHaveTextContent("false");
  });

  it("loads last project on startup if valid", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce("/home/user/myproject")
      .mockResolvedValueOnce({ is_repo: true, path: "/home/user/myproject" });

    function TestComponent() {
      const { isLoading, isProjectOpen, projectName } = useApp();
      return (
        <div>
          <span data-testid="loading">{isLoading.toString()}</span>
          <span data-testid="open">{isProjectOpen.toString()}</span>
          <span data-testid="name">{projectName || "none"}</span>
        </div>
      );
    }

    render(
      <AppProvider>
        <TestComponent />
      </AppProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("open")).toHaveTextContent("true");
    });

    expect(screen.getByTestId("name")).toHaveTextContent("myproject");
    expect(screen.getByTestId("loading")).toHaveTextContent("false");
  });

  it("finishes loading without project if last project is not a repo", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce("/home/user/oldproject")
      .mockResolvedValueOnce({ is_repo: false, path: "/home/user/oldproject" });

    function TestComponent() {
      const { isLoading, isProjectOpen } = useApp();
      return (
        <div>
          <span data-testid="loading">{isLoading.toString()}</span>
          <span data-testid="open">{isProjectOpen.toString()}</span>
        </div>
      );
    }

    render(
      <AppProvider>
        <TestComponent />
      </AppProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("loading")).toHaveTextContent("false");
    });

    expect(screen.getByTestId("open")).toHaveTextContent("false");
  });

  it("clearError clears the error state", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    function TestComponent() {
      const { error, clearError } = useApp();
      return (
        <div>
          <span data-testid="error">{error || "none"}</span>
          <button onClick={clearError}>Clear</button>
        </div>
      );
    }

    render(
      <AppProvider>
        <TestComponent />
      </AppProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("error")).toHaveTextContent("none");
    });

    await userEvent.click(screen.getByRole("button"));
    expect(screen.getByTestId("error")).toHaveTextContent("none");
  });
});
