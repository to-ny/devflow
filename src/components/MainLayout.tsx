import { Group, Panel, Separator } from "react-resizable-panels";
import { ErrorBoundary } from "./ErrorBoundary";
import { FileTree } from "./FileTree";
import { DiffView } from "./DiffView";
import { ChatPanel } from "./ChatPanel";
import "./MainLayout.css";

export function MainLayout() {
  return (
    <div className="main-layout">
      <Group orientation="horizontal" className="panel-group">
        <Panel defaultSize={20} minSize={15} className="panel">
          <ErrorBoundary>
            <FileTree />
          </ErrorBoundary>
        </Panel>

        <Separator className="resize-handle" />

        <Panel defaultSize={50} minSize={30} className="panel">
          <ErrorBoundary>
            <DiffView />
          </ErrorBoundary>
        </Panel>

        <Separator className="resize-handle" />

        <Panel defaultSize={30} minSize={20} className="panel">
          <ErrorBoundary>
            <ChatPanel />
          </ErrorBoundary>
        </Panel>
      </Group>
    </div>
  );
}
