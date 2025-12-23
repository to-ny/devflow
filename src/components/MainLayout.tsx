import { Group, Panel, Separator } from "react-resizable-panels";
import { FileTree } from "./FileTree";
import { DiffView } from "./DiffView";
import { ChatPanel } from "./ChatPanel";
import "./MainLayout.css";

export function MainLayout() {
  return (
    <div className="main-layout">
      <Group orientation="horizontal" className="panel-group">
        <Panel defaultSize={20} minSize={15} className="panel">
          <FileTree />
        </Panel>

        <Separator className="resize-handle" />

        <Panel defaultSize={50} minSize={30} className="panel">
          <DiffView />
        </Panel>

        <Separator className="resize-handle" />

        <Panel defaultSize={30} minSize={20} className="panel">
          <ChatPanel />
        </Panel>
      </Group>
    </div>
  );
}
