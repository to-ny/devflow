import "./Panel.css";

export function DiffView() {
  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2>Diff</h2>
      </div>
      <div className="panel-content">
        <p className="placeholder-text">Select a file to view changes</p>
      </div>
    </div>
  );
}
