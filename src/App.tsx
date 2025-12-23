import { AppProvider, useApp } from "./context/AppContext";
import { CommentsProvider } from "./context/CommentsContext";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { MainLayout } from "./components/MainLayout";
import "./App.css";

function AppContent() {
  const { isProjectOpen } = useApp();

  return isProjectOpen ? <MainLayout /> : <WelcomeScreen />;
}

function App() {
  return (
    <AppProvider>
      <CommentsProvider>
        <AppContent />
      </CommentsProvider>
    </AppProvider>
  );
}

export default App;
