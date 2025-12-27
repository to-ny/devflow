import { SettingsSection } from "./SettingsSection";
import type { FormValidationErrors } from "../../hooks/useSettingsForm";
import type { ProviderInfo, ProjectConfig } from "../../types/config";

interface AgentSectionProps {
  config: ProjectConfig;
  providers: ProviderInfo[];
  currentModels: string[];
  validationErrors: FormValidationErrors;
  onProviderChange: (providerId: string) => void;
  onUpdateAgent: (field: string, value: string | number | null) => void;
}

export function AgentSection({
  config,
  providers,
  currentModels,
  validationErrors,
  onProviderChange,
  onUpdateAgent,
}: AgentSectionProps) {
  const { agent } = config;
  const isCustomModel = !currentModels.includes(agent.model);

  return (
    <SettingsSection
      title="Agent Configuration"
      description="Configure the AI provider and model settings"
    >
      <div className="form-group">
        <label htmlFor="provider">Provider</label>
        <select
          id="provider"
          value={agent.provider}
          onChange={(e) => onProviderChange(e.target.value)}
        >
          {providers.map((provider) => (
            <option key={provider.id} value={provider.id}>
              {provider.name}
            </option>
          ))}
        </select>
      </div>

      <div
        className={`form-group ${validationErrors.model ? "has-error" : ""}`}
      >
        <label htmlFor="model">Model</label>
        <select
          id="model-select"
          value={isCustomModel ? "__custom__" : agent.model}
          onChange={(e) => {
            const value = e.target.value;
            if (value !== "__custom__") {
              onUpdateAgent("model", value);
            } else {
              onUpdateAgent("model", "");
            }
          }}
        >
          {currentModels.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
          <option value="__custom__">Other (custom)</option>
        </select>
        {isCustomModel && (
          <input
            type="text"
            id="model"
            value={agent.model}
            onChange={(e) => onUpdateAgent("model", e.target.value)}
            placeholder="Enter custom model name"
            className="custom-model-input"
          />
        )}
        {validationErrors.model && (
          <span className="field-error">{validationErrors.model}</span>
        )}
      </div>

      <div
        className={`form-group ${validationErrors.api_key_env ? "has-error" : ""}`}
      >
        <label htmlFor="api_key_env">API Key Environment Variable</label>
        <input
          type="text"
          id="api_key_env"
          value={agent.api_key_env}
          onChange={(e) => onUpdateAgent("api_key_env", e.target.value)}
        />
        {validationErrors.api_key_env && (
          <span className="field-error">{validationErrors.api_key_env}</span>
        )}
      </div>

      <div
        className={`form-group ${validationErrors.max_tokens ? "has-error" : ""}`}
      >
        <label htmlFor="max_tokens">Max Tokens</label>
        <input
          type="number"
          id="max_tokens"
          value={agent.max_tokens}
          onChange={(e) => onUpdateAgent("max_tokens", Number(e.target.value))}
          min={1}
          max={200000}
        />
        {validationErrors.max_tokens && (
          <span className="field-error">{validationErrors.max_tokens}</span>
        )}
      </div>

      <div className="form-group">
        <label htmlFor="context_limit">Context Limit (tokens)</label>
        <input
          type="number"
          id="context_limit"
          value={agent.context_limit ?? ""}
          onChange={(e) => {
            const value = e.target.value;
            onUpdateAgent("context_limit", value === "" ? null : Number(value));
          }}
          min={10000}
          max={2000000}
          placeholder="200000 (default)"
        />
        <span className="field-hint">
          Maximum context window size. Leave empty to use default (200k).
        </span>
      </div>
    </SettingsSection>
  );
}
