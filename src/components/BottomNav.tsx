import { useNavigation } from "../context/NavigationContext";
import { useChat } from "../context/ChatContext";
import "./BottomNav.css";

function formatTokenCount(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return count.toString();
}

function formatCharCount(count: number): string {
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return count.toString();
}

export function BottomNav() {
  const { currentPage, navigate } = useNavigation();
  const { sessionUsage, memoryInfo } = useChat();

  const totalTokens = sessionUsage.input_tokens + sessionUsage.output_tokens;
  const hasUsage = totalTokens > 0;

  const showLeftIndicators = memoryInfo || hasUsage;

  return (
    <nav className="bottom-nav" aria-label="Main navigation">
      {showLeftIndicators && (
        <div className="bottom-nav-left">
          {memoryInfo && (
            <div
              className={`memory-indicator ${memoryInfo.truncated ? "truncated" : ""}`}
              title={`AGENTS.md loaded (${formatCharCount(memoryInfo.charCount)} chars)${memoryInfo.truncated ? " - truncated" : ""}`}
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
                <line x1="16" y1="13" x2="8" y2="13" />
                <line x1="16" y1="17" x2="8" y2="17" />
                <polyline points="10 9 9 9 8 9" />
              </svg>
              <span>AGENTS.md</span>
            </div>
          )}
          {hasUsage && (
            <div
              className="token-counter"
              title={`Input: ${sessionUsage.input_tokens.toLocaleString()} | Output: ${sessionUsage.output_tokens.toLocaleString()}`}
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <circle cx="12" cy="12" r="10" />
                <path d="M12 6v6l4 2" />
              </svg>
              <span>{formatTokenCount(totalTokens)}</span>
            </div>
          )}
        </div>
      )}
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
