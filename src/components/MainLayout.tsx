import { useNavigation } from "../context/NavigationContext";
import { useSession } from "../context/SessionContext";
import { BottomNav } from "./BottomNav";
import { Toast } from "./Toast";
import { ChatPage, ChangesPage, SettingsPage } from "../pages";
import "./MainLayout.css";

export function MainLayout() {
  const { currentPage } = useNavigation();
  const { memoryWarning, clearMemoryWarning } = useSession();

  return (
    <div className="main-layout">
      <div className="page-content">
        {currentPage === "chat" && <ChatPage />}
        {currentPage === "changes" && <ChangesPage />}
        {currentPage === "settings" && <SettingsPage />}
      </div>
      <BottomNav />
      {memoryWarning && (
        <Toast
          message={memoryWarning}
          type="warning"
          onDismiss={clearMemoryWarning}
        />
      )}
    </div>
  );
}
