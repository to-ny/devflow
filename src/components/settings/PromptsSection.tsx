import { useState } from "react";
import { SettingsSection } from "./SettingsSection";
import { MarkdownEditor } from "./MarkdownEditor";
import type { ProjectConfig } from "../../types/config";
import { isPresent } from "../../utils/nullish";

type PromptType = "system" | "extraction" | "pre" | "post" | "agents-md";

const PROMPT_OPTIONS: { id: PromptType; label: string }[] = [
  { id: "system", label: "System Prompt" },
  { id: "extraction", label: "Extraction Prompt" },
  { id: "pre", label: "Pre-prompt" },
  { id: "post", label: "Post-prompt" },
  { id: "agents-md", label: "AGENTS.md" },
];

interface PromptsSectionProps {
  config: ProjectConfig;
  defaultSystemPrompt: string;
  defaultExtractionPrompt: string;
  agentsMd: string | null;
  onUpdateSystemPrompt: (value: string) => void;
  onResetSystemPrompt: () => void;
  onUpdateExtractionPrompt: (value: string) => void;
  onResetExtractionPrompt: () => void;
  onUpdatePrePostPrompt: (field: "pre" | "post", value: string) => void;
  onResetPrePostPrompt: (field: "pre" | "post") => void;
  onSetAgentsMd: (value: string | null) => void;
}

export function PromptsSection({
  config,
  defaultSystemPrompt,
  defaultExtractionPrompt,
  agentsMd,
  onUpdateSystemPrompt,
  onResetSystemPrompt,
  onUpdateExtractionPrompt,
  onResetExtractionPrompt,
  onUpdatePrePostPrompt,
  onResetPrePostPrompt,
  onSetAgentsMd,
}: PromptsSectionProps) {
  const [selectedPrompt, setSelectedPrompt] = useState<PromptType>("system");

  const getPromptValue = (type: PromptType): string => {
    if (type === "system") {
      return config.system_prompt ?? defaultSystemPrompt;
    }
    if (type === "extraction") {
      return config.extraction_prompt ?? defaultExtractionPrompt;
    }
    if (type === "agents-md") {
      return agentsMd || "";
    }
    return config.prompts[type] || "";
  };

  const isPromptCustom = (type: PromptType): boolean => {
    if (type === "agents-md") return false;
    if (type === "system") return isPresent(config.system_prompt);
    if (type === "extraction") return isPresent(config.extraction_prompt);
    return (config.prompts[type] || "").length > 0;
  };

  const promptHasDefault = (type: PromptType): boolean => {
    return type !== "agents-md";
  };

  const handlePromptChange = (type: PromptType, value: string) => {
    if (type === "system") {
      onUpdateSystemPrompt(value);
    } else if (type === "extraction") {
      onUpdateExtractionPrompt(value);
    } else if (type === "agents-md") {
      onSetAgentsMd(value);
    } else {
      onUpdatePrePostPrompt(type, value);
    }
  };

  const handlePromptReset = (type: PromptType) => {
    if (type === "system") {
      onResetSystemPrompt();
    } else if (type === "extraction") {
      onResetExtractionPrompt();
    } else if (type !== "agents-md") {
      onResetPrePostPrompt(type);
    }
  };

  return (
    <SettingsSection
      title="Prompts"
      description="Configure system and project-specific prompts"
      fullWidth
    >
      <div className="override-header">
        <div className="tool-selector">
          <select
            value={selectedPrompt}
            onChange={(e) => setSelectedPrompt(e.target.value as PromptType)}
          >
            {PROMPT_OPTIONS.map((opt) => (
              <option key={opt.id} value={opt.id}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        {promptHasDefault(selectedPrompt) && (
          <div className="override-status">
            <span
              className={`override-badge ${isPromptCustom(selectedPrompt) ? "custom" : "default"}`}
            >
              {isPromptCustom(selectedPrompt) ? "Custom" : "Default"}
            </span>
            {isPromptCustom(selectedPrompt) && (
              <button
                className="reset-button"
                onClick={() => handlePromptReset(selectedPrompt)}
              >
                Reset to Default
              </button>
            )}
          </div>
        )}
      </div>
      <div className="editor-container">
        <MarkdownEditor
          key={selectedPrompt}
          value={getPromptValue(selectedPrompt)}
          onChange={(value) => handlePromptChange(selectedPrompt, value)}
          placeholder={
            selectedPrompt === "system"
              ? "System instructions for the AI..."
              : selectedPrompt === "extraction"
                ? "Prompt for extracting key facts during context compaction..."
                : selectedPrompt === "pre"
                  ? "Instructions prepended to each request..."
                  : selectedPrompt === "post"
                    ? "Instructions appended to each request..."
                    : "# Project Memory\n\nAdd project-specific context here..."
          }
        />
      </div>
    </SettingsSection>
  );
}
