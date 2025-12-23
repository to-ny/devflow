import { useApp } from "../context/AppContext";
import "./WelcomeScreen.css";

export function WelcomeScreen() {
  const { openProject, error, clearError, isLoading } = useApp();

  if (isLoading) {
    return (
      <div className="welcome-screen">
        <div className="welcome-content">
          <h1 className="welcome-title">Devflow</h1>
          <p className="welcome-subtitle">Loading...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="welcome-screen">
      <div className="welcome-content">
        <h1 className="welcome-title">Devflow</h1>
        <p className="welcome-subtitle">
          AI-assisted iterative code development
        </p>
        {error && (
          <div className="error-message" onClick={clearError}>
            {error}
          </div>
        )}
        <button className="open-project-btn" onClick={openProject}>
          Open Project
        </button>
      </div>
    </div>
  );
}
