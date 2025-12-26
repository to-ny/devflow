import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { ReactNode } from "react";
import { NavigationProvider, useNavigation, Page } from "./NavigationContext";
import { listen } from "@tauri-apps/api/event";

// Store event listeners for simulation
type EventCallback<T> = (event: { payload: T }) => void;
const eventListeners: Map<string, EventCallback<unknown>> = new Map();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, callback: EventCallback<unknown>) => {
    eventListeners.set(eventName, callback);
    return Promise.resolve(() => {
      eventListeners.delete(eventName);
    });
  }),
}));

// Helper to simulate menu navigation events
function simulateMenuNavigate(page: string) {
  const callback = eventListeners.get("menu-navigate");
  if (callback) {
    callback({ payload: page });
  }
}

function createWrapper() {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <NavigationProvider>{children}</NavigationProvider>;
  };
}

describe("useNavigation", () => {
  it("throws error when used outside NavigationProvider", () => {
    expect(() => renderHook(() => useNavigation())).toThrow(
      "useNavigation must be used within a NavigationProvider",
    );
  });
});

describe("NavigationProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventListeners.clear();
  });

  afterEach(() => {
    eventListeners.clear();
  });

  describe("initial state", () => {
    it("sets initial page to chat", () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      expect(result.current.currentPage).toBe("chat");
    });

    it("sets up menu-navigate listener on mount", async () => {
      renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(listen).toHaveBeenCalledWith(
          "menu-navigate",
          expect.any(Function),
        );
      });
    });
  });

  describe("navigate", () => {
    it("navigates to chat page", () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.navigate("changes");
      });

      expect(result.current.currentPage).toBe("changes");

      act(() => {
        result.current.navigate("chat");
      });

      expect(result.current.currentPage).toBe("chat");
    });

    it("navigates to changes page", () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.navigate("changes");
      });

      expect(result.current.currentPage).toBe("changes");
    });

    it("navigates to settings page", () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.navigate("settings");
      });

      expect(result.current.currentPage).toBe("settings");
    });

    it("allows navigation between all pages", () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      const pages: Page[] = ["chat", "changes", "settings"];

      for (const page of pages) {
        act(() => {
          result.current.navigate(page);
        });
        expect(result.current.currentPage).toBe(page);
      }
    });
  });

  describe("menu-navigate event handling", () => {
    it("updates page on valid menu-navigate event", async () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(eventListeners.has("menu-navigate")).toBe(true);
      });

      act(() => {
        simulateMenuNavigate("settings");
      });

      expect(result.current.currentPage).toBe("settings");
    });

    it("navigates to chat via menu event", async () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(eventListeners.has("menu-navigate")).toBe(true);
      });

      // First go to a different page
      act(() => {
        result.current.navigate("changes");
      });

      expect(result.current.currentPage).toBe("changes");

      // Navigate via menu event
      act(() => {
        simulateMenuNavigate("chat");
      });

      expect(result.current.currentPage).toBe("chat");
    });

    it("navigates to changes via menu event", async () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(eventListeners.has("menu-navigate")).toBe(true);
      });

      act(() => {
        simulateMenuNavigate("changes");
      });

      expect(result.current.currentPage).toBe("changes");
    });

    it("ignores invalid page values from menu event", async () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(eventListeners.has("menu-navigate")).toBe(true);
      });

      const initialPage = result.current.currentPage;

      act(() => {
        simulateMenuNavigate("invalid-page");
      });

      // Should remain on the initial page
      expect(result.current.currentPage).toBe(initialPage);
    });

    it("ignores non-string payloads from menu event", async () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(eventListeners.has("menu-navigate")).toBe(true);
      });

      const initialPage = result.current.currentPage;

      // Simulate with invalid payload types
      const callback = eventListeners.get("menu-navigate");
      if (callback) {
        act(() => {
          callback({ payload: 123 });
        });
        act(() => {
          callback({ payload: null });
        });
        act(() => {
          callback({ payload: undefined });
        });
        act(() => {
          callback({ payload: {} });
        });
      }

      // Should remain on the initial page
      expect(result.current.currentPage).toBe(initialPage);
    });
  });

  describe("page type validation", () => {
    it("only accepts valid Page types", () => {
      const { result } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      // TypeScript ensures only valid pages can be passed to navigate
      const validPages: Page[] = ["chat", "changes", "settings"];

      validPages.forEach((page) => {
        act(() => {
          result.current.navigate(page);
        });
        expect(result.current.currentPage).toBe(page);
      });
    });
  });

  describe("state persistence", () => {
    it("maintains page state across re-renders", () => {
      const { result, rerender } = renderHook(() => useNavigation(), {
        wrapper: createWrapper(),
      });

      act(() => {
        result.current.navigate("settings");
      });

      rerender();

      expect(result.current.currentPage).toBe("settings");
    });
  });
});
