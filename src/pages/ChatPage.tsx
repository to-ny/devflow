import { ChatPanel } from "../components/ChatPanel";
import { ErrorBoundary } from "../components/ErrorBoundary";
import "./ChatPage.css";

export function ChatPage() {
  return (
    <div className="chat-page">
      <ErrorBoundary>
        <ChatPanel />
      </ErrorBoundary>
    </div>
  );
}
