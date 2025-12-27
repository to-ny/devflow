import { AppProvider, useApp } from "./context/AppContext";
import { SessionProvider } from "./context/SessionContext";
import { ChatProvider } from "./context/ChatContext";
import { CommentsProvider } from "./context/CommentsContext";
import { NavigationProvider } from "./context/NavigationContext";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { MainLayout } from "./components/MainLayout";
import "./App.css";

function AppContent() {
  const { isProjectOpen, projectPath } = useApp();

  if (!isProjectOpen) {
    return <WelcomeScreen />;
  }

  return (
    <NavigationProvider>
      <SessionProvider projectPath={projectPath}>
        <ChatProvider projectPath={projectPath}>
          <CommentsProvider>
            <MainLayout />
          </CommentsProvider>
        </ChatProvider>
      </SessionProvider>
    </NavigationProvider>
  );
}

function App() {
  return (
    <AppProvider>
      <AppContent />
    </AppProvider>
  );
}

export default App;
