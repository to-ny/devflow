import { SettingsSection } from "./SettingsSection";
import type { NotificationAction, ProjectConfig } from "../../types/config";

interface NotificationsSectionProps {
  config: ProjectConfig;
  onUpdate: (
    type: "on_complete" | "on_error",
    action: NotificationAction,
    checked: boolean,
  ) => void;
}

export function NotificationsSection({
  config,
  onUpdate,
}: NotificationsSectionProps) {
  const { notifications } = config;

  return (
    <SettingsSection
      title="Notifications"
      description="Configure notification preferences"
    >
      <div className="form-group">
        <span className="form-label">On Complete</span>
        <div className="checkbox-group">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={notifications.on_complete?.includes("sound") || false}
              onChange={(e) =>
                onUpdate("on_complete", "sound", e.target.checked)
              }
            />
            Sound
          </label>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={notifications.on_complete?.includes("window") || false}
              onChange={(e) =>
                onUpdate("on_complete", "window", e.target.checked)
              }
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
              checked={notifications.on_error?.includes("sound") || false}
              onChange={(e) => onUpdate("on_error", "sound", e.target.checked)}
            />
            Sound
          </label>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={notifications.on_error?.includes("window") || false}
              onChange={(e) => onUpdate("on_error", "window", e.target.checked)}
            />
            Flash Window
          </label>
        </div>
      </div>
    </SettingsSection>
  );
}
