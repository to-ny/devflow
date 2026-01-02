import { useState, useMemo } from "react";
import { SettingsSection } from "./SettingsSection";
import { MarkdownEditor } from "./MarkdownEditor";
import type { ProjectConfig } from "../../types/config";
import type { AgentTypeInfo } from "../../hooks/useSettingsForm";
import { isPresent } from "../../utils/nullish";

interface AgentPromptsSectionProps {
  config: ProjectConfig;
  defaultAgentPrompts: Record<string, string>;
  agentTypes: AgentTypeInfo[];
  onUpdateAgentPrompt: (agentId: string, value: string) => void;
  onResetAgentPrompt: (agentId: string) => void;
}

export function AgentPromptsSection({
  config,
  defaultAgentPrompts,
  agentTypes,
  onUpdateAgentPrompt,
  onResetAgentPrompt,
}: AgentPromptsSectionProps) {
  const [selectedAgent, setSelectedAgent] = useState<string>(() => {
    return agentTypes[0]?.id || "";
  });

  const agentOptions = useMemo(() => {
    return agentTypes.map((agent) => ({
      id: agent.id,
      label: agent.name,
    }));
  }, [agentTypes]);

  const getPromptValue = (agentId: string): string => {
    return (
      config.agent_prompts?.[agentId] ?? defaultAgentPrompts[agentId] ?? ""
    );
  };

  const isCustom = (agentId: string): boolean => {
    return isPresent(config.agent_prompts?.[agentId]);
  };

  return (
    <SettingsSection
      title="Agent Prompts"
      description="Configure prompts for specialized sub-agents"
      fullWidth
    >
      <div className="override-header">
        <div className="tool-selector">
          <select
            value={selectedAgent}
            onChange={(e) => setSelectedAgent(e.target.value)}
          >
            {agentOptions.map((opt) => (
              <option key={opt.id} value={opt.id}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="override-status">
          <span
            className={`override-badge ${isCustom(selectedAgent) ? "custom" : "default"}`}
          >
            {isCustom(selectedAgent) ? "Custom" : "Default"}
          </span>
          {isCustom(selectedAgent) && (
            <button
              className="reset-button"
              onClick={() => onResetAgentPrompt(selectedAgent)}
            >
              Reset to Default
            </button>
          )}
        </div>
      </div>
      <div className="editor-container">
        <MarkdownEditor
          key={selectedAgent}
          value={getPromptValue(selectedAgent)}
          onChange={(value) => onUpdateAgentPrompt(selectedAgent, value)}
          placeholder="Agent system prompt..."
        />
      </div>
    </SettingsSection>
  );
}
