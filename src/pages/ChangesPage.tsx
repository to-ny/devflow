import { useEffect } from "react";
import { Panel, Group, Separator } from "react-resizable-panels";
import { FileTree } from "../components/FileTree";
import { DiffView } from "../components/DiffView";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { useApp } from "../context/AppContext";
import "../components/Panel.css";
import "./ChangesPage.css";

export function ChangesPage() {
  const { refreshFiles, isProjectOpen } = useApp();

  useEffect(() => {
    if (isProjectOpen) {
      refreshFiles();
    }
  }, [isProjectOpen, refreshFiles]);

  return (
    <div className="changes-page">
      <Group orientation="horizontal" className="changes-panel-group">
        <Panel defaultSize={25} minSize={15} className="resizable-panel">
          <ErrorBoundary>
            <FileTree />
          </ErrorBoundary>
        </Panel>

        <Separator className="resize-handle" />

        <Panel defaultSize={75} minSize={40} className="resizable-panel">
          <ErrorBoundary>
            <DiffView />
          </ErrorBoundary>
        </Panel>
      </Group>
    </div>
  );
}
