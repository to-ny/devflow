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
import type { ChangedFile, FileStatus } from "../types/git";

interface AppState {
  projectPath: string | null;
  projectName: string | null;
  isProjectOpen: boolean;
  isLoading: boolean;
  error: string | null;
  changedFiles: ChangedFile[];
  selectedFile: string | null;
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
  selectFile: (path: string | null) => void;
  refreshFiles: () => Promise<void>;
  getSelectedFileInfo: () => ChangedFile | null;
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
    changedFiles: [],
    selectedFile: null,
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
    } catch {
      // Ignored
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
        changedFiles: [],
        selectedFile: null,
      });

      updateWindowTitle(projectName);
      invoke("config_set_last_project", { projectPath }).catch(() => {});
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
    } catch {
      if (isMounted.current) {
        setState((prev) => ({
          ...prev,
          error: "Failed to open project",
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
      changedFiles: [],
      selectedFile: null,
    });

    updateWindowTitle(null);
    invoke("config_set_last_project", { projectPath: null }).catch(() => {});
  }, [updateWindowTitle]);

  const clearError = useCallback(() => {
    setState((prev) => ({ ...prev, error: null }));
  }, []);

  const selectFile = useCallback((path: string | null) => {
    setState((prev) => ({ ...prev, selectedFile: path }));
  }, []);

  const getSelectedFileInfo = useCallback((): ChangedFile | null => {
    if (!state.selectedFile) return null;
    return (
      state.changedFiles.find((f) => f.path === state.selectedFile) ?? null
    );
  }, [state.selectedFile, state.changedFiles]);

  // Not memoized intentionally - needs current state.projectPath
  const refreshFiles = async () => {
    if (!state.projectPath) {
      return;
    }

    try {
      const files = await invoke<ChangedFile[]>("git_get_changed_files", {
        projectPath: state.projectPath,
      });

      if (isMounted.current && files) {
        setState((prev) => {
          const selectedStillExists = files.some(
            (f) => f.path === prev.selectedFile,
          );
          return {
            ...prev,
            changedFiles: files,
            selectedFile: selectedStillExists ? prev.selectedFile : null,
          };
        });
      }
    } catch {
      // Ignored
    }
  };

  useEffect(() => {
    if (state.isProjectOpen && state.projectPath) {
      refreshFiles();
    }
  }, [state.isProjectOpen, state.projectPath]);

  useEffect(() => {
    let cancelled = false;

    async function loadLastProject() {
      try {
        const lastProject = await invoke<string | null>(
          "config_get_last_project",
        );

        if (cancelled) return;

        if (lastProject) {
          const result = await invoke<RepoCheckResult>("git_is_repository", {
            path: lastProject,
          });

          if (cancelled) return;

          if (result.is_repo) {
            setProjectOpen(lastProject);
            return;
          }
        }
      } catch {
        // Ignored
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
      } catch {
        // Ignored
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
        selectFile,
        refreshFiles,
        getSelectedFileInfo,
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

export function getFileStatusForDiff(file: ChangedFile): {
  indexStatus: FileStatus | null;
  worktreeStatus: FileStatus | null;
} {
  return {
    indexStatus: file.index_status,
    worktreeStatus: file.worktree_status,
  };
}
