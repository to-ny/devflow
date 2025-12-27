import { SettingsSection } from "./SettingsSection";
import type { FormValidationErrors } from "../../hooks/useSettingsForm";
import type { ProjectConfig } from "../../types/config";

interface SearchSectionProps {
  config: ProjectConfig;
  validationErrors: FormValidationErrors;
  onUpdate: (field: string, value: number) => void;
}

export function SearchSection({
  config,
  validationErrors,
  onUpdate,
}: SearchSectionProps) {
  const { search } = config;

  return (
    <SettingsSection
      title="Search Settings"
      description="Configure web search behavior"
    >
      <div
        className={`form-group ${validationErrors.search_max_results ? "has-error" : ""}`}
      >
        <label htmlFor="search_max_results">Max Search Results</label>
        <input
          type="number"
          id="search_max_results"
          value={search.max_results}
          onChange={(e) => onUpdate("max_results", Number(e.target.value))}
          min={1}
          max={50}
        />
        {validationErrors.search_max_results && (
          <span className="field-error">
            {validationErrors.search_max_results}
          </span>
        )}
      </div>
    </SettingsSection>
  );
}
