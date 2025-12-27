import "./SettingsSidebar.css";

export interface SidebarItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
}

interface SettingsSidebarProps {
  items: SidebarItem[];
  activeId: string;
  onSelect: (id: string) => void;
}

export function SettingsSidebar({
  items,
  activeId,
  onSelect,
}: SettingsSidebarProps) {
  return (
    <nav className="settings-sidebar">
      {items.map((item) => (
        <button
          key={item.id}
          className={`sidebar-item ${activeId === item.id ? "active" : ""}`}
          onClick={() => onSelect(item.id)}
        >
          {item.icon && <span className="sidebar-icon">{item.icon}</span>}
          <span className="sidebar-label">{item.label}</span>
        </button>
      ))}
    </nav>
  );
}
