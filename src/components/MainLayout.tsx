import { useNavigation } from "../context/NavigationContext";
import { BottomNav } from "./BottomNav";
import { ChatPage, ChangesPage, SettingsPage } from "../pages";
import "./MainLayout.css";

export function MainLayout() {
  const { currentPage } = useNavigation();

  return (
    <div className="main-layout">
      <div className="page-content">
        {currentPage === "chat" && <ChatPage />}
        {currentPage === "changes" && <ChangesPage />}
        {currentPage === "settings" && <SettingsPage />}
      </div>
      <BottomNav />
    </div>
  );
}
