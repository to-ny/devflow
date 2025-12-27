import { ReactNode } from "react";
import "./SettingsSection.css";

interface SettingsSectionProps {
  title: string;
  description?: string;
  children: ReactNode;
  fullWidth?: boolean;
}

export function SettingsSection({
  title,
  description,
  children,
  fullWidth,
}: SettingsSectionProps) {
  return (
    <section
      className={`settings-section-card ${fullWidth ? "full-width" : ""}`}
    >
      <div className="section-header">
        <h3 className="section-title">{title}</h3>
        {description && <p className="section-description">{description}</p>}
      </div>
      <div className="section-content">{children}</div>
    </section>
  );
}
