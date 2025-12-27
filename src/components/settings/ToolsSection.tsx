import { useState, useEffect } from "react";
import { SettingsSection } from "./SettingsSection";
import { MarkdownEditor } from "./MarkdownEditor";
import type { ProjectConfig } from "../../types/config";

interface ToolsSectionProps {
  config: ProjectConfig;
  defaultToolDescriptions: Record<string, string>;
  onUpdateToolDescription: (tool: string, value: string) => void;
  onResetToolDescription: (tool: string) => void;
}

export function ToolsSection({
  config,
  defaultToolDescriptions,
  onUpdateToolDescription,
  onResetToolDescription,
}: ToolsSectionProps) {
  const toolNames = Object.keys(defaultToolDescriptions);
  const [selectedTool, setSelectedTool] = useState<string>("");

  useEffect(() => {
    if (toolNames.length > 0 && !selectedTool) {
      setSelectedTool(toolNames[0]);
    }
  }, [toolNames, selectedTool]);

  const selectedToolCustomDesc = config.tool_descriptions?.[selectedTool];
  const isSelectedToolCustom = selectedToolCustomDesc !== undefined;
  const selectedToolValue =
    selectedToolCustomDesc ?? defaultToolDescriptions[selectedTool] ?? "";

  return (
    <SettingsSection
      title="Tool Descriptions"
      description="Customize the descriptions shown to the AI for each tool"
      fullWidth
    >
      <div className="override-header">
        <div className="tool-selector">
          <select
            value={selectedTool}
            onChange={(e) => setSelectedTool(e.target.value)}
          >
            {toolNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </div>
        <div className="override-status">
          <span
            className={`override-badge ${isSelectedToolCustom ? "custom" : "default"}`}
          >
            {isSelectedToolCustom ? "Custom" : "Default"}
          </span>
          {isSelectedToolCustom && (
            <button
              className="reset-button"
              onClick={() => onResetToolDescription(selectedTool)}
            >
              Reset to Default
            </button>
          )}
        </div>
      </div>
      <div className="editor-container">
        <MarkdownEditor
          key={selectedTool}
          value={selectedToolValue}
          onChange={(value) => onUpdateToolDescription(selectedTool, value)}
        />
      </div>
    </SettingsSection>
  );
}
