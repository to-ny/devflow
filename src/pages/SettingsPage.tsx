import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../context/AppContext";
import type { ProjectConfig, NotificationAction } from "../types/config";
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
    },
    notifications: {
      on_complete,
      on_error,
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
    a.on_complete_sound === b.on_complete_sound &&
    a.on_complete_window === b.on_complete_window &&
    a.on_error_sound === b.on_error_sound &&
    a.on_error_window === b.on_error_window
  );
}

export function SettingsPage() {
  const { projectPath } = useApp();
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
  const hasValidationErrors = Object.keys(validationErrors).length > 0;

  useEffect(() => {
    if (!projectPath) return;

    async function loadConfig() {
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
    }

    loadConfig();
  }, [projectPath]);

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
          <h2>Project Settings</h2>
          {isDirty && (
            <div className="settings-unsaved">You have unsaved changes</div>
          )}
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
                onChange={handleChange}
              >
                <option value="anthropic">Anthropic</option>
                <option value="gemini">Gemini</option>
              </select>
            </div>
            <div
              className={`form-group ${validationErrors.model ? "has-error" : ""}`}
            >
              <label htmlFor="model">Model</label>
              <input
                type="text"
                id="model"
                name="model"
                value={form.model}
                onChange={handleChange}
              />
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
            className="save-button"
            onClick={handleSave}
            disabled={isSaving || !isDirty || hasValidationErrors}
          >
            {isSaving ? "Saving..." : "Save Settings"}
          </button>
        </div>
      </div>
    </div>
  );
}
