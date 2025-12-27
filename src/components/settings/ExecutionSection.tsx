import { SettingsSection } from "./SettingsSection";
import type { FormValidationErrors } from "../../hooks/useSettingsForm";
import type { ProjectConfig } from "../../types/config";

interface ExecutionSectionProps {
  config: ProjectConfig;
  validationErrors: FormValidationErrors;
  onUpdate: (field: string, value: number) => void;
}

export function ExecutionSection({
  config,
  validationErrors,
  onUpdate,
}: ExecutionSectionProps) {
  const { execution } = config;

  return (
    <SettingsSection
      title="Execution Settings"
      description="Configure tool execution limits and timeouts"
    >
      <div
        className={`form-group ${validationErrors.timeout_secs ? "has-error" : ""}`}
      >
        <label htmlFor="timeout_secs">Tool Timeout (seconds)</label>
        <input
          type="number"
          id="timeout_secs"
          value={execution.timeout_secs}
          onChange={(e) => onUpdate("timeout_secs", Number(e.target.value))}
          min={1}
          max={600}
        />
        {validationErrors.timeout_secs && (
          <span className="field-error">{validationErrors.timeout_secs}</span>
        )}
      </div>

      <div
        className={`form-group ${validationErrors.max_tool_iterations ? "has-error" : ""}`}
      >
        <label htmlFor="max_tool_iterations">Max Tool Iterations</label>
        <input
          type="number"
          id="max_tool_iterations"
          value={execution.max_tool_iterations}
          onChange={(e) =>
            onUpdate("max_tool_iterations", Number(e.target.value))
          }
          min={1}
          max={1000}
        />
        {validationErrors.max_tool_iterations && (
          <span className="field-error">
            {validationErrors.max_tool_iterations}
          </span>
        )}
      </div>

      <div
        className={`form-group ${validationErrors.max_agent_depth ? "has-error" : ""}`}
      >
        <label htmlFor="max_agent_depth">Max Sub-agent Depth</label>
        <input
          type="number"
          id="max_agent_depth"
          value={execution.max_agent_depth}
          onChange={(e) => onUpdate("max_agent_depth", Number(e.target.value))}
          min={1}
          max={10}
        />
        {validationErrors.max_agent_depth && (
          <span className="field-error">
            {validationErrors.max_agent_depth}
          </span>
        )}
      </div>
    </SettingsSection>
  );
}
