import {
  createContext,
  useContext,
  useState,
  useEffect,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";

export type Page = "chat" | "changes" | "settings";

interface NavigationContextType {
  currentPage: Page;
  navigate: (page: Page) => void;
}

const NavigationContext = createContext<NavigationContextType | null>(null);

interface NavigationProviderProps {
  children: ReactNode;
}

function isValidPage(value: unknown): value is Page {
  return value === "chat" || value === "changes" || value === "settings";
}

export function NavigationProvider({ children }: NavigationProviderProps) {
  const [currentPage, setCurrentPage] = useState<Page>("chat");

  const navigate = (page: Page) => {
    setCurrentPage(page);
  };

  useEffect(() => {
    const unlisten = listen<string>("menu-navigate", (event) => {
      if (isValidPage(event.payload)) {
        setCurrentPage(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <NavigationContext.Provider value={{ currentPage, navigate }}>
      {children}
    </NavigationContext.Provider>
  );
}

export function useNavigation(): NavigationContextType {
  const context = useContext(NavigationContext);
  if (!context) {
    throw new Error("useNavigation must be used within a NavigationProvider");
  }
  return context;
}
