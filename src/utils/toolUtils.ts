// Shared utility functions for tool display

export function getToolIcon(name: string): string {
  switch (name) {
    case "bash":
      return "\u2318"; // Command symbol
    case "read_file":
      return "\uD83D\uDCC4"; // Page emoji
    case "write_file":
      return "\u270F\uFE0F"; // Pencil emoji
    case "edit_file":
      return "\uD83D\uDCDD"; // Memo emoji
    case "multi_edit":
      return "\uD83D\uDCDD"; // Memo emoji
    case "list_directory":
      return "\uD83D\uDCC1"; // Folder emoji
    case "glob":
      return "\uD83D\uDD0D"; // Magnifying glass
    case "grep":
      return "\uD83D\uDD0E"; // Right magnifying glass
    case "web_fetch":
      return "\uD83C\uDF10"; // Globe emoji
    case "search_web":
      return "\uD83D\uDD0D"; // Magnifying glass
    case "dispatch_agent":
      return "\uD83E\uDD16"; // Robot emoji
    case "submit_plan":
      return "\uD83D\uDCCB"; // Clipboard emoji
    case "todo_read":
      return "\u2705"; // Check mark
    case "todo_write":
      return "\u270F\uFE0F"; // Pencil
    case "notebook_read":
      return "\uD83D\uDCD3"; // Notebook
    case "notebook_edit":
      return "\uD83D\uDCD4"; // Notebook with pen
    default:
      return "\uD83D\uDD27"; // Wrench emoji
  }
}

export function getToolLabel(name: string): string {
  switch (name) {
    case "bash":
      return "Shell Command";
    case "read_file":
      return "Read File";
    case "write_file":
      return "Write File";
    case "edit_file":
      return "Edit File";
    case "multi_edit":
      return "Multi Edit";
    case "list_directory":
      return "List Directory";
    case "glob":
      return "Find Files";
    case "grep":
      return "Search Content";
    case "web_fetch":
      return "Fetch URL";
    case "search_web":
      return "Web Search";
    case "dispatch_agent":
      return "Sub-Agent";
    case "submit_plan":
      return "Submit Plan";
    case "todo_read":
      return "Read Todos";
    case "todo_write":
      return "Update Todos";
    case "notebook_read":
      return "Read Notebook";
    case "notebook_edit":
      return "Edit Notebook";
    default:
      return name;
  }
}
