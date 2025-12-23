import { AppProvider, useApp } from "./context/AppContext";
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
      <AppContent />
    </AppProvider>
  );
}

export default App;
