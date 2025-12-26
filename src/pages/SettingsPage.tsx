import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useApp } from "../context/AppContext";
import { useLatest } from "../hooks/useLatest";
import type {
  ConfigChangedPayload,
  NotificationAction,
  ProjectConfig,
  ProviderInfo,
} from "../types/config";
import "./SettingsPage.css";

interface FormState {
  // Agent
  provider: string;
  model: string;
  api_key_env: string;
  max_tokens: number;
  // Prompts
  pre: string;
  post: string;
  // Execution
  timeout_secs: number;
  max_tool_iterations: number;
  max_agent_depth: number;
  // Search
  search_max_results: number;
  // Notifications
  on_complete_sound: boolean;
  on_complete_window: boolean;
  on_error_sound: boolean;
  on_error_window: boolean;
}

interface ValidationErrors {
  model?: string;
  api_key_env?: string;
  max_tokens?: string;
  timeout_secs?: string;
  max_tool_iterations?: string;
  max_agent_depth?: string;
  search_max_results?: string;
}

const defaultFormState: FormState = {
  provider: "anthropic",
  model: "claude-sonnet-4-20250514",
  api_key_env: "ANTHROPIC_API_KEY",
  max_tokens: 8192,
  pre: "",
  post: "",
  timeout_secs: 30,
  max_tool_iterations: 50,
  max_agent_depth: 3,
  search_max_results: 10,
  on_complete_sound: false,
  on_complete_window: false,
  on_error_sound: false,
  on_error_window: false,
};

function configToFormState(config: ProjectConfig): FormState {
  return {
    provider: config.agent.provider,
    model: config.agent.model,
    api_key_env: config.agent.api_key_env,
    max_tokens: config.agent.max_tokens,
    pre: config.prompts.pre,
    post: config.prompts.post,
    timeout_secs: Number(config.execution.timeout_secs),
    max_tool_iterations: config.execution.max_tool_iterations,
    max_agent_depth: config.execution.max_agent_depth,
    search_max_results: config.search.max_results,
    on_complete_sound: config.notifications.on_complete.includes("sound"),
    on_complete_window: config.notifications.on_complete.includes("window"),
    on_error_sound: config.notifications.on_error.includes("sound"),
    on_error_window: config.notifications.on_error.includes("window"),
  };
}

function formStateToConfig(form: FormState): ProjectConfig {
  const on_complete: NotificationAction[] = [];
  const on_error: NotificationAction[] = [];

  if (form.on_complete_sound) on_complete.push("sound");
  if (form.on_complete_window) on_complete.push("window");
  if (form.on_error_sound) on_error.push("sound");
  if (form.on_error_window) on_error.push("window");

  return {
    agent: {
      provider: form.provider,
      model: form.model,
      api_key_env: form.api_key_env,
      max_tokens: form.max_tokens,
    },
    prompts: {
      pre: form.pre,
      post: form.post,
    },
    execution: {
      timeout_secs: form.timeout_secs,
      max_tool_iterations: form.max_tool_iterations,
      max_agent_depth: form.max_agent_depth,
    },
    notifications: {
      on_complete,
      on_error,
    },
    search: {
      provider: "duckduckgo",
      max_results: form.search_max_results,
    },
  };
}

function validateForm(form: FormState): ValidationErrors {
  const errors: ValidationErrors = {};

  if (!form.model.trim()) {
    errors.model = "Model is required";
  }

  if (!form.api_key_env.trim()) {
    errors.api_key_env = "API key environment variable is required";
  }

  if (form.max_tokens < 1 || form.max_tokens > 200000) {
    errors.max_tokens = "Max tokens must be between 1 and 200,000";
  }

  if (form.timeout_secs < 1 || form.timeout_secs > 600) {
    errors.timeout_secs = "Timeout must be between 1 and 600 seconds";
  }

  if (form.max_tool_iterations < 1 || form.max_tool_iterations > 1000) {
    errors.max_tool_iterations = "Max iterations must be between 1 and 1,000";
  }

  if (form.max_agent_depth < 1 || form.max_agent_depth > 10) {
    errors.max_agent_depth = "Max agent depth must be between 1 and 10";
  }

  if (form.search_max_results < 1 || form.search_max_results > 50) {
    errors.search_max_results = "Max results must be between 1 and 50";
  }

  return errors;
}

function areFormsEqual(a: FormState, b: FormState): boolean {
  return (
    a.provider === b.provider &&
    a.model === b.model &&
    a.api_key_env === b.api_key_env &&
    a.max_tokens === b.max_tokens &&
    a.pre === b.pre &&
    a.post === b.post &&
    a.timeout_secs === b.timeout_secs &&
    a.max_tool_iterations === b.max_tool_iterations &&
    a.max_agent_depth === b.max_agent_depth &&
    a.search_max_results === b.search_max_results &&
    a.on_complete_sound === b.on_complete_sound &&
    a.on_complete_window === b.on_complete_window &&
    a.on_error_sound === b.on_error_sound &&
    a.on_error_window === b.on_error_window
  );
}

export function SettingsPage() {
  const { projectPath } = useApp();
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [form, setForm] = useState<FormState>(defaultFormState);
  const [savedForm, setSavedForm] = useState<FormState>(defaultFormState);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [validationErrors, setValidationErrors] = useState<ValidationErrors>(
    {},
  );

  const isDirty = useMemo(
    () => !areFormsEqual(form, savedForm),
    [form, savedForm],
  );
  const isDirtyRef = useLatest(isDirty);
  const hasValidationErrors = Object.keys(validationErrors).length > 0;

  // Fetch available providers on mount
  useEffect(() => {
    invoke<ProviderInfo[]>("config_get_providers").then(setProviders);
  }, []);

  const loadConfig = useCallback(async () => {
    if (!projectPath) return;
    try {
      setIsLoading(true);
      setError(null);
      const config = await invoke<ProjectConfig>("config_load_project", {
        projectPath,
      });
      const formState = configToFormState(config);
      setForm(formState);
      setSavedForm(formState);
    } catch (e) {
      setError(`Failed to load config: ${e}`);
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  // Load config on mount and project change
  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // Listen for external config changes
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ConfigChangedPayload>("config-changed", (event) => {
      if (!isDirtyRef.current && event.payload.project_path === projectPath) {
        loadConfig();
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [projectPath, loadConfig]);

  // Warn on page unload if there are unsaved changes
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (isDirty) {
        e.preventDefault();
      }
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [isDirty]);

  const handleChange = useCallback(
    (
      e: React.ChangeEvent<
        HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
      >,
    ) => {
      const { name, value, type } = e.target;
      const checked = (e.target as HTMLInputElement).checked;

      setForm((prev) => ({
        ...prev,
        [name]:
          type === "checkbox"
            ? checked
            : type === "number"
              ? Number(value)
              : value,
      }));
      setSuccessMessage(null);

      // Clear validation error for this field when user starts typing
      if (validationErrors[name as keyof ValidationErrors]) {
        setValidationErrors((prev) => {
          const next = { ...prev };
          delete next[name as keyof ValidationErrors];
          return next;
        });
      }
    },
    [validationErrors],
  );

  const currentProvider = useMemo(
    () => providers.find((p) => p.id === form.provider),
    [providers, form.provider],
  );

  const currentModels = useMemo(
    () => currentProvider?.models || [],
    [currentProvider],
  );

  const handleProviderChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const providerId = e.target.value;
      const provider = providers.find((p) => p.id === providerId);

      setForm((prev) => ({
        ...prev,
        provider: providerId,
        model: provider?.models[0] || "",
        api_key_env: provider?.default_api_key_env || "",
      }));
      setSuccessMessage(null);
    },
    [providers],
  );

  const isCustomModel = useMemo(
    () => !currentModels.includes(form.model),
    [currentModels, form.model],
  );

  const handleModelSelectChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const value = e.target.value;
      if (value !== "__custom__") {
        setForm((prev) => ({ ...prev, model: value }));
        setSuccessMessage(null);
      } else {
        setForm((prev) => ({ ...prev, model: "" }));
        setSuccessMessage(null);
      }
    },
    [],
  );

  const handleSave = async () => {
    if (!projectPath) return;

    // Validate before saving
    const errors = validateForm(form);
    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      setError("Please fix the validation errors before saving");
      return;
    }

    try {
      setIsSaving(true);
      setError(null);
      setSuccessMessage(null);
      setValidationErrors({});
      const config = formStateToConfig(form);
      await invoke("config_save_project", { projectPath, config });
      setSavedForm(form);
      setSuccessMessage("Settings saved successfully");
    } catch (e) {
      setError(`Failed to save: ${e}`);
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading) {
    return (
      <div className="settings-page">
        <div className="settings-loading">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className="settings-page">
      <div className="settings-content">
        <div className="settings-header">
          <h2>
            Project Settings
            {isDirty && (
              <span className="unsaved-indicator" title="Unsaved changes" />
            )}
          </h2>
          {error && <div className="settings-error">{error}</div>}
          {successMessage && (
            <div className="settings-success">{successMessage}</div>
          )}
        </div>

        <div className="settings-sections">
          <section className="settings-section">
            <h3>Agent</h3>
            <div className="form-group">
              <label htmlFor="provider">Provider</label>
              <select
                id="provider"
                name="provider"
                value={form.provider}
                onChange={handleProviderChange}
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
                value={isCustomModel ? "__custom__" : form.model}
                onChange={handleModelSelectChange}
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
                  name="model"
                  value={form.model}
                  onChange={handleChange}
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
                name="api_key_env"
                value={form.api_key_env}
                onChange={handleChange}
              />
              {validationErrors.api_key_env && (
                <span className="field-error">
                  {validationErrors.api_key_env}
                </span>
              )}
            </div>
            <div
              className={`form-group ${validationErrors.max_tokens ? "has-error" : ""}`}
            >
              <label htmlFor="max_tokens">Max Tokens</label>
              <input
                type="number"
                id="max_tokens"
                name="max_tokens"
                value={form.max_tokens}
                onChange={handleChange}
                min={1}
                max={200000}
              />
              {validationErrors.max_tokens && (
                <span className="field-error">
                  {validationErrors.max_tokens}
                </span>
              )}
            </div>
          </section>

          <section className="settings-section">
            <h3>Prompts</h3>
            <div className="form-group">
              <label htmlFor="pre">Pre-prompt</label>
              <textarea
                id="pre"
                name="pre"
                value={form.pre}
                onChange={handleChange}
                rows={4}
                placeholder="System instructions prepended to each request..."
              />
            </div>
            <div className="form-group">
              <label htmlFor="post">Post-prompt</label>
              <textarea
                id="post"
                name="post"
                value={form.post}
                onChange={handleChange}
                rows={4}
                placeholder="Instructions appended to each request..."
              />
            </div>
          </section>

          <section className="settings-section">
            <h3>Execution</h3>
            <div
              className={`form-group ${validationErrors.timeout_secs ? "has-error" : ""}`}
            >
              <label htmlFor="timeout_secs">Tool Timeout (seconds)</label>
              <input
                type="number"
                id="timeout_secs"
                name="timeout_secs"
                value={form.timeout_secs}
                onChange={handleChange}
                min={1}
                max={600}
              />
              {validationErrors.timeout_secs && (
                <span className="field-error">
                  {validationErrors.timeout_secs}
                </span>
              )}
            </div>
            <div
              className={`form-group ${validationErrors.max_tool_iterations ? "has-error" : ""}`}
            >
              <label htmlFor="max_tool_iterations">Max Tool Iterations</label>
              <input
                type="number"
                id="max_tool_iterations"
                name="max_tool_iterations"
                value={form.max_tool_iterations}
                onChange={handleChange}
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
                name="max_agent_depth"
                value={form.max_agent_depth}
                onChange={handleChange}
                min={1}
                max={10}
              />
              {validationErrors.max_agent_depth && (
                <span className="field-error">
                  {validationErrors.max_agent_depth}
                </span>
              )}
            </div>
          </section>

          <section className="settings-section">
            <h3>Search</h3>
            <div
              className={`form-group ${validationErrors.search_max_results ? "has-error" : ""}`}
            >
              <label htmlFor="search_max_results">Max Search Results</label>
              <input
                type="number"
                id="search_max_results"
                name="search_max_results"
                value={form.search_max_results}
                onChange={handleChange}
                min={1}
                max={50}
              />
              {validationErrors.search_max_results && (
                <span className="field-error">
                  {validationErrors.search_max_results}
                </span>
              )}
            </div>
          </section>

          <section className="settings-section">
            <h3>Notifications</h3>
            <div className="form-group">
              <span className="form-label">On Complete</span>
              <div className="checkbox-group">
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    name="on_complete_sound"
                    checked={form.on_complete_sound}
                    onChange={handleChange}
                  />
                  Sound
                </label>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    name="on_complete_window"
                    checked={form.on_complete_window}
                    onChange={handleChange}
                  />
                  Flash Window
                </label>
              </div>
            </div>
            <div className="form-group">
              <span className="form-label">On Error</span>
              <div className="checkbox-group">
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    name="on_error_sound"
                    checked={form.on_error_sound}
                    onChange={handleChange}
                  />
                  Sound
                </label>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    name="on_error_window"
                    checked={form.on_error_window}
                    onChange={handleChange}
                  />
                  Flash Window
                </label>
              </div>
            </div>
          </section>
        </div>

        <div className="settings-footer">
          <button
            className={`save-button ${!isDirty && !isSaving ? "saved" : ""}`}
            onClick={handleSave}
            disabled={isSaving || !isDirty || hasValidationErrors}
          >
            {isSaving ? "Saving..." : isDirty ? "Save Settings" : "Saved"}
          </button>
        </div>
      </div>
    </div>
  );
}
