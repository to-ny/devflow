import { useState, useEffect } from "react";
import { useApp } from "../context/AppContext";
import { useLatest } from "../hooks/useLatest";
import { useSettingsForm } from "../hooks/useSettingsForm";
import {
  SettingsSidebar,
  AgentSection,
  ExecutionSection,
  SearchSection,
  NotificationsSection,
  PromptsSection,
  ToolsSection,
  type SidebarItem,
} from "../components/settings";
import "./SettingsPage.css";

interface SidebarSection {
  id: string;
  label: string;
}

const SECTIONS: SidebarSection[] = [
  { id: "agent", label: "Agent" },
  { id: "execution", label: "Execution" },
  { id: "search", label: "Search" },
  { id: "notifications", label: "Notifications" },
  { id: "prompts", label: "Prompts" },
  { id: "tools", label: "Tool Descriptions" },
];

export function SettingsPage() {
  const { projectPath } = useApp();
  const [activeSection, setActiveSection] = useState("agent");

  const form = useSettingsForm({ projectPath });
  const isDirtyRef = useLatest(form.isDirty);

  // Warn on page unload if dirty
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (isDirtyRef.current) {
        e.preventDefault();
      }
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [isDirtyRef]);

  const sidebarItems: SidebarItem[] = SECTIONS.map((s) => ({
    id: s.id,
    label: s.label,
  }));

  const hasValidationErrors = Object.keys(form.validationErrors).length > 0;

  if (form.isLoading) {
    return (
      <div className="settings-page">
        <div className="settings-loading">Loading settings...</div>
      </div>
    );
  }

  if (form.error && !form.projectConfig) {
    return (
      <div className="settings-page">
        <div className="settings-error">{form.error}</div>
      </div>
    );
  }

  if (!projectPath || !form.projectConfig) {
    return (
      <div className="settings-page">
        <div className="settings-loading">Open a project to edit settings</div>
      </div>
    );
  }

  const renderSectionContent = () => {
    switch (activeSection) {
      case "agent":
        return (
          <AgentSection
            config={form.projectConfig!}
            providers={form.providers}
            currentModels={form.currentModels}
            validationErrors={form.validationErrors}
            onProviderChange={form.handleProviderChange}
            onUpdateAgent={form.updateAgent}
          />
        );

      case "execution":
        return (
          <ExecutionSection
            config={form.projectConfig!}
            validationErrors={form.validationErrors}
            onUpdate={form.updateExecution}
          />
        );

      case "search":
        return (
          <SearchSection
            config={form.projectConfig!}
            validationErrors={form.validationErrors}
            onUpdate={form.updateSearch}
          />
        );

      case "notifications":
        return (
          <NotificationsSection
            config={form.projectConfig!}
            onUpdate={form.updateNotifications}
          />
        );

      case "prompts":
        return (
          <PromptsSection
            config={form.projectConfig!}
            defaultSystemPrompt={form.defaultSystemPrompt}
            agentsMd={form.agentsMd}
            onUpdateSystemPrompt={form.updateSystemPrompt}
            onResetSystemPrompt={form.resetSystemPrompt}
            onUpdatePrePostPrompt={form.updatePrePostPrompt}
            onResetPrePostPrompt={form.resetPrePostPrompt}
            onSetAgentsMd={form.setAgentsMd}
          />
        );

      case "tools":
        return (
          <ToolsSection
            config={form.projectConfig!}
            defaultToolDescriptions={form.defaultToolDescriptions}
            onUpdateToolDescription={form.updateToolDescription}
            onResetToolDescription={form.resetToolDescription}
          />
        );

      default:
        return null;
    }
  };

  return (
    <div className="settings-page">
      <div className="settings-layout">
        <SettingsSidebar
          items={sidebarItems}
          activeId={activeSection}
          onSelect={setActiveSection}
        />

        <div className="settings-main">
          <div className="settings-header">
            {form.error && <div className="settings-error">{form.error}</div>}
            {form.successMessage && (
              <div className="settings-success">{form.successMessage}</div>
            )}
          </div>

          <div className="settings-content-area">{renderSectionContent()}</div>

          <div className="settings-footer">
            <button
              className={`save-button ${!form.isDirty && !form.isSaving ? "saved" : ""}`}
              onClick={form.handleSave}
              disabled={form.isSaving || !form.isDirty || hasValidationErrors}
            >
              {form.isSaving
                ? "Saving..."
                : form.isDirty
                  ? "Save Settings"
                  : "Saved"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
