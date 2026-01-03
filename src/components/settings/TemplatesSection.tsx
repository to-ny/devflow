import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SettingsSection } from "./SettingsSection";
import { MarkdownEditor } from "./MarkdownEditor";
import type { TemplateContent } from "../../types/generated";

type TemplateType = "review-comments" | "commit";

const TEMPLATE_OPTIONS: { id: TemplateType; label: string }[] = [
  { id: "review-comments", label: "Review Comments" },
  { id: "commit", label: "Commit" },
];

export function TemplatesSection() {
  const [selectedTemplate, setSelectedTemplate] =
    useState<TemplateType>("review-comments");
  const [templates, setTemplates] = useState<TemplateContent | null>(null);
  const [savedTemplates, setSavedTemplates] = useState<TemplateContent | null>(
    null,
  );
  const [defaults, setDefaults] = useState<TemplateContent | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const isDirty =
    templates && savedTemplates
      ? JSON.stringify(templates) !== JSON.stringify(savedTemplates)
      : false;

  useEffect(() => {
    const loadTemplates = async () => {
      try {
        setIsLoading(true);
        const [loaded, defaultTemplates] = await Promise.all([
          invoke<TemplateContent>("template_load"),
          invoke<TemplateContent>("template_get_defaults"),
        ]);
        setTemplates(loaded);
        setSavedTemplates(loaded);
        setDefaults(defaultTemplates);
      } catch (e) {
        setError(`Failed to load templates: ${e}`);
      } finally {
        setIsLoading(false);
      }
    };
    loadTemplates();
  }, []);

  const getTemplateValue = (type: TemplateType): string => {
    if (!templates) return "";
    return type === "review-comments"
      ? templates.review_comments
      : templates.commit;
  };

  const isTemplateCustom = (type: TemplateType): boolean => {
    if (!templates || !defaults) return false;
    const current =
      type === "review-comments" ? templates.review_comments : templates.commit;
    const defaultVal =
      type === "review-comments" ? defaults.review_comments : defaults.commit;
    return current !== defaultVal;
  };

  const handleTemplateChange = useCallback(
    (type: TemplateType, value: string) => {
      setTemplates((prev) => {
        if (!prev) return prev;
        if (type === "review-comments") {
          return { ...prev, review_comments: value };
        } else {
          return { ...prev, commit: value };
        }
      });
      setSuccessMessage(null);
    },
    [],
  );

  const handleTemplateReset = useCallback(
    (type: TemplateType) => {
      if (!defaults) return;
      setTemplates((prev) => {
        if (!prev) return prev;
        if (type === "review-comments") {
          return { ...prev, review_comments: defaults.review_comments };
        } else {
          return { ...prev, commit: defaults.commit };
        }
      });
      setSuccessMessage(null);
    },
    [defaults],
  );

  const handleSave = useCallback(async () => {
    if (!templates) return;
    try {
      setIsSaving(true);
      setError(null);
      await invoke("template_save", { templates });
      setSavedTemplates(templates);
      setSuccessMessage("Templates saved successfully");
    } catch (e) {
      setError(`Failed to save templates: ${e}`);
    } finally {
      setIsSaving(false);
    }
  }, [templates]);

  if (isLoading) {
    return (
      <SettingsSection
        title="Templates"
        description="Configure message templates for review comments and commits"
        fullWidth
      >
        <div className="settings-loading">Loading templates...</div>
      </SettingsSection>
    );
  }

  return (
    <SettingsSection
      title="Templates"
      description="Configure message templates for review comments and commits (global, not per-project)"
      fullWidth
    >
      {error && <div className="settings-error">{error}</div>}
      {successMessage && (
        <div className="settings-success">{successMessage}</div>
      )}

      <div className="override-header">
        <div className="tool-selector">
          <select
            value={selectedTemplate}
            onChange={(e) =>
              setSelectedTemplate(e.target.value as TemplateType)
            }
          >
            {TEMPLATE_OPTIONS.map((opt) => (
              <option key={opt.id} value={opt.id}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="override-status">
          <span
            className={`override-badge ${isTemplateCustom(selectedTemplate) ? "custom" : "default"}`}
          >
            {isTemplateCustom(selectedTemplate) ? "Custom" : "Default"}
          </span>
          {isTemplateCustom(selectedTemplate) && (
            <button
              className="reset-button"
              onClick={() => handleTemplateReset(selectedTemplate)}
            >
              Reset to Default
            </button>
          )}
        </div>
      </div>

      <div className="editor-container">
        <MarkdownEditor
          key={selectedTemplate}
          value={getTemplateValue(selectedTemplate)}
          onChange={(value) => handleTemplateChange(selectedTemplate, value)}
          placeholder={
            selectedTemplate === "review-comments"
              ? "Template for sending review comments to the agent..."
              : "Template for commit instructions..."
          }
        />
      </div>

      <div className="template-variables">
        <strong>Available variables:</strong>
        {selectedTemplate === "review-comments" ? (
          <code>
            {"{{global_comment}}"}, {"{{#each comments}}"}, {"{{file}}"},{" "}
            {"{{lines.start}}"}, {"{{lines.end}}"}, {"{{selected_code}}"},{" "}
            {"{{text}}"}
          </code>
        ) : (
          <code>{"{{instructions}}"}</code>
        )}
      </div>

      <div className="template-save-container">
        <button
          className={`save-button ${!isDirty && !isSaving ? "saved" : ""}`}
          onClick={handleSave}
          disabled={isSaving || !isDirty}
        >
          {isSaving ? "Saving..." : isDirty ? "Save Templates" : "Saved"}
        </button>
      </div>
    </SettingsSection>
  );
}
