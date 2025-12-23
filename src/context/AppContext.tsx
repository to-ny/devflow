import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
  ReactNode,
} from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

interface AppState {
  projectPath: string | null;
  projectName: string | null;
  isProjectOpen: boolean;
  isLoading: boolean;
  error: string | null;
}

interface RepoCheckResult {
  is_repo: boolean;
  path: string;
  exists: boolean;
  is_dir: boolean;
  error: string | null;
}

interface AppContextValue extends AppState {
  openProject: () => Promise<void>;
  closeProject: () => void;
  clearError: () => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function extractFolderName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AppState>({
    projectPath: null,
    projectName: null,
    isProjectOpen: false,
    isLoading: true,
    error: null,
  });

  // Track if component is mounted to avoid state updates after unmount
  const isMounted = useRef(true);

  useEffect(() => {
    isMounted.current = true;
    return () => {
      isMounted.current = false;
    };
  }, []);

  const updateWindowTitle = useCallback(async (projectName: string | null) => {
    const title = projectName ? `Devflow - ${projectName}` : "Devflow";
    try {
      await getCurrentWindow().setTitle(title);
    } catch (err) {
      console.error("Failed to update window title:", err);
    }
  }, []);

  const setProjectOpen = useCallback(
    (projectPath: string) => {
      const projectName = extractFolderName(projectPath);
      setState({
        projectPath,
        projectName,
        isProjectOpen: true,
        isLoading: false,
        error: null,
      });

      // Update window title asynchronously
      updateWindowTitle(projectName);

      // Persist last project
      invoke("config_set_last_project", { projectPath }).catch(console.error);
    },
    [updateWindowTitle],
  );

  const openProject = useCallback(async () => {
    setState((prev) => ({ ...prev, error: null }));

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Project Directory",
      });

      if (selected && typeof selected === "string") {
        // Validate it's a git repository
        const result = await invoke<RepoCheckResult>("git_is_repository", {
          path: selected,
        });

        if (!result.is_repo) {
          if (isMounted.current) {
            const errorMsg = result.error
              ? `Not a git repository: ${result.error}`
              : "Selected folder is not a git repository";
            setState((prev) => ({
              ...prev,
              error: errorMsg,
            }));
          }
          return;
        }

        setProjectOpen(selected);
      }
    } catch (err) {
      console.error("Failed to open project:", err);
      if (isMounted.current) {
        const errorMessage = "Failed to open project";
        setState((prev) => ({
          ...prev,
          error: errorMessage,
        }));
      }
    }
  }, [setProjectOpen]);

  const closeProject = useCallback(() => {
    setState({
      projectPath: null,
      projectName: null,
      isProjectOpen: false,
      isLoading: false,
      error: null,
    });

    // Update window title asynchronously
    updateWindowTitle(null);

    // Clear last project
    invoke("config_set_last_project", { projectPath: null }).catch(
      console.error,
    );
  }, [updateWindowTitle]);

  const clearError = useCallback(() => {
    setState((prev) => ({ ...prev, error: null }));
  }, []);

  // Load last project on startup
  useEffect(() => {
    let cancelled = false;

    async function loadLastProject() {
      try {
        const lastProject = await invoke<string | null>(
          "config_get_last_project",
        );

        if (cancelled) return;

        if (lastProject) {
          // Verify it's still a valid git repository
          const result = await invoke<RepoCheckResult>("git_is_repository", {
            path: lastProject,
          });

          if (cancelled) return;

          if (result.is_repo) {
            setProjectOpen(lastProject);
            return;
          }
        }
      } catch (err) {
        console.error("Failed to load last project:", err);
      }

      if (!cancelled) {
        setState((prev) => ({ ...prev, isLoading: false }));
      }
    }

    loadLastProject();

    return () => {
      cancelled = true;
    };
  }, [setProjectOpen]);

  // Listen for menu events
  useEffect(() => {
    let cancelled = false;
    const unlisteners: (() => void)[] = [];

    async function setupListeners() {
      try {
        const unlistenOpen = await listen("menu-open-project", () => {
          openProject();
        });
        const unlistenClose = await listen("menu-close-project", () => {
          closeProject();
        });

        if (cancelled) {
          unlistenOpen();
          unlistenClose();
        } else {
          unlisteners.push(unlistenOpen, unlistenClose);
        }
      } catch (err) {
        console.error("Failed to setup menu listeners:", err);
      }
    }

    setupListeners();

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [openProject, closeProject]);

  return (
    <AppContext.Provider
      value={{
        ...state,
        openProject,
        closeProject,
        clearError,
      }}
    >
      {children}
    </AppContext.Provider>
  );
}

export function useApp() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useApp must be used within an AppProvider");
  }
  return context;
}
