import { useNavigation } from "../context/NavigationContext";
import "./BottomNav.css";

export function BottomNav() {
  const { currentPage, navigate } = useNavigation();

  return (
    <nav className="bottom-nav" aria-label="Main navigation">
      <div className="bottom-nav-items">
        <NavButton
          label="Chat"
          isActive={currentPage === "chat"}
          onClick={() => navigate("chat")}
          icon={
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          }
        />
        <NavButton
          label="Changes"
          isActive={currentPage === "changes"}
          onClick={() => navigate("changes")}
          icon={
            <>
              <path d="M6 3v12" />
              <circle cx="18" cy="6" r="3" />
              <circle cx="6" cy="18" r="3" />
              <path d="M18 9a9 9 0 0 1-9 9" />
            </>
          }
        />
        <NavButton
          label="Settings"
          isActive={currentPage === "settings"}
          onClick={() => navigate("settings")}
          icon={
            <>
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </>
          }
        />
      </div>
    </nav>
  );
}

interface NavButtonProps {
  label: string;
  isActive: boolean;
  onClick: () => void;
  icon: React.ReactNode;
}

function NavButton({ label, isActive, onClick, icon }: NavButtonProps) {
  return (
    <button
      className={`bottom-nav-item ${isActive ? "active" : ""}`}
      onClick={onClick}
      title={label}
      aria-label={`Navigate to ${label}`}
      aria-current={isActive ? "page" : undefined}
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        aria-hidden="true"
      >
        {icon}
      </svg>
      <span>{label}</span>
    </button>
  );
}
