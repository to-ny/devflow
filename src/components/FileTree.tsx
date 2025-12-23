import "./Panel.css";

export function FileTree() {
  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2>Files</h2>
      </div>
      <div className="panel-content">
        <p className="placeholder-text">Changed files will appear here</p>
      </div>
    </div>
  );
}
