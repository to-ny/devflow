import "./Panel.css";

export function ChatPanel() {
  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2>Chat</h2>
      </div>
      <div className="panel-content">
        <p className="placeholder-text">Chat with the AI agent</p>
      </div>
    </div>
  );
}
