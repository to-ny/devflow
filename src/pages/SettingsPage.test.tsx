import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsPage } from "./SettingsPage";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectConfig, ProviderInfo } from "../types/config";

// Mock Tauri APIs
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

// Mock AppContext
vi.mock("../context/AppContext", () => ({
  useApp: () => ({
    projectPath: "/test/project",
  }),
}));

// Default mock data
const mockProviders: ProviderInfo[] = [
  {
    id: "anthropic",
    name: "Anthropic",
    models: ["claude-sonnet-4-20250514", "claude-opus-4-20250514"],
    default_api_key_env: "ANTHROPIC_API_KEY",
  },
  {
    id: "gemini",
    name: "Gemini",
    models: ["gemini-2.0-flash", "gemini-1.5-pro"],
    default_api_key_env: "GEMINI_API_KEY",
  },
];

const mockConfig: ProjectConfig = {
  agent: {
    provider: "anthropic",
    model: "claude-sonnet-4-20250514",
    api_key_env: "ANTHROPIC_API_KEY",
    max_tokens: 8192,
    context_limit: null,
  },
  prompts: {
    pre: "Pre-prompt text",
    post: "Post-prompt text",
  },
  execution: {
    timeout_secs: 30,
    max_tool_iterations: 50,
    max_agent_depth: 3,
  },
  notifications: {
    on_complete: ["sound"],
    on_error: ["sound", "window"],
  },
  search: {
    provider: "duckduckgo",
    max_results: 10,
  },
  system_prompt: null,
  extraction_prompt: null,
  tool_descriptions: null,
  agent_prompts: null,
};

const mockDefaultSystemPrompt = "You are a helpful assistant.";

const mockToolDescriptions: Record<string, string> = {
  bash: "Execute bash commands",
  read_file: "Read file contents",
};

const mockAgentPrompts: Record<string, string> = {
  explore: "Explore agent prompt",
  plan: "Plan agent prompt",
};

const mockAgentTypes = [
  { id: "explore", name: "Explore", description: "Codebase exploration" },
  { id: "plan", name: "Plan", description: "Implementation planning" },
];

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "config_get_providers") {
        return Promise.resolve(mockProviders);
      }
      if (cmd === "config_load_project") {
        return Promise.resolve(mockConfig);
      }
      if (cmd === "config_save_project") {
        return Promise.resolve(undefined);
      }
      if (cmd === "config_get_default_system_prompt") {
        return Promise.resolve(mockDefaultSystemPrompt);
      }
      if (cmd === "config_get_default_extraction_prompt") {
        return Promise.resolve("Default extraction prompt");
      }
      if (cmd === "config_get_tool_descriptions") {
        return Promise.resolve(mockToolDescriptions);
      }
      if (cmd === "config_get_agent_prompts") {
        return Promise.resolve(mockAgentPrompts);
      }
      if (cmd === "config_get_agent_types") {
        return Promise.resolve(mockAgentTypes);
      }
      if (cmd === "config_load_agents_md") {
        return Promise.resolve(null);
      }
      if (cmd === "config_save_agents_md") {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });
  });

  describe("loading state", () => {
    it("shows loading state initially", () => {
      render(<SettingsPage />);
      expect(screen.getByText("Loading settings...")).toBeInTheDocument();
    });

    it("loads and displays settings after loading", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Check Agent Configuration section is shown (default)
      expect(screen.getByText("Agent Configuration")).toBeInTheDocument();
    });
  });

  describe("sidebar navigation", () => {
    it("displays sidebar sections", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Check for sidebar by finding the nav element with sidebar items
      const sidebar = document.querySelector(".settings-sidebar");
      expect(sidebar).toBeInTheDocument();

      // Check that Agent and Execution sections exist within sidebar
      const agentButtons = screen.getAllByRole("button", { name: /Agent/i });
      expect(agentButtons.length).toBeGreaterThan(0);

      const executionButton = screen.getByRole("button", {
        name: /Execution/i,
      });
      expect(executionButton).toBeInTheDocument();
    });
  });

  describe("form population", () => {
    it("populates provider field from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const providerSelect = screen.getByLabelText(
        "Provider",
      ) as HTMLSelectElement;
      expect(providerSelect.value).toBe("anthropic");
    });

    it("populates model field from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const modelSelect = document.getElementById(
        "model-select",
      ) as HTMLSelectElement;
      expect(modelSelect.value).toBe("claude-sonnet-4-20250514");
    });
  });

  describe("provider selection", () => {
    it("updates model options when provider changes", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const providerSelect = screen.getByLabelText("Provider");
      await userEvent.selectOptions(providerSelect, "gemini");

      // Check model select has Gemini models
      const modelSelect = document.getElementById(
        "model-select",
      ) as HTMLSelectElement;
      expect(modelSelect.value).toBe("gemini-2.0-flash");
    });

    it("updates API key env when provider changes", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const providerSelect = screen.getByLabelText("Provider");
      await userEvent.selectOptions(providerSelect, "gemini");

      const apiKeyInput = screen.getByLabelText(
        "API Key Environment Variable",
      ) as HTMLInputElement;
      expect(apiKeyInput.value).toBe("GEMINI_API_KEY");
    });
  });

  describe("error handling", () => {
    it("shows error when config load fails", async () => {
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "config_get_providers") {
          return Promise.resolve(mockProviders);
        }
        if (cmd === "config_get_default_system_prompt") {
          return Promise.resolve(mockDefaultSystemPrompt);
        }
        if (cmd === "config_get_tool_descriptions") {
          return Promise.resolve(mockToolDescriptions);
        }
        if (cmd === "config_load_agents_md") {
          return Promise.resolve(null);
        }
        if (cmd === "config_load_project") {
          return Promise.reject(new Error("Failed to load config"));
        }
        return Promise.resolve(undefined);
      });

      render(<SettingsPage />);

      await waitFor(() => {
        expect(screen.getByText(/Failed to load config/)).toBeInTheDocument();
      });
    });
  });

  describe("custom model input", () => {
    it("shows custom model input when Other is selected", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const modelSelect = document.getElementById(
        "model-select",
      ) as HTMLSelectElement;
      await userEvent.selectOptions(modelSelect, "__custom__");

      expect(
        screen.getByPlaceholderText("Enter custom model name"),
      ).toBeInTheDocument();
    });

    it("allows entering custom model name", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const modelSelect = document.getElementById(
        "model-select",
      ) as HTMLSelectElement;
      await userEvent.selectOptions(modelSelect, "__custom__");

      const customInput = screen.getByPlaceholderText(
        "Enter custom model name",
      );
      await userEvent.type(customInput, "custom-model-v1");

      expect((customInput as HTMLInputElement).value).toBe("custom-model-v1");
    });
  });

  describe("dirty tracking", () => {
    it("save button is initially disabled when no changes", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Initially saved and disabled
      const saveButton = screen.getByRole("button", { name: "Saved" });
      expect(saveButton).toBeDisabled();
    });

    it("enables save button when changes are made", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Make a change by switching provider
      const providerSelect = screen.getByLabelText("Provider");
      await userEvent.selectOptions(providerSelect, "gemini");

      // Save button should be enabled
      await waitFor(() => {
        const saveButton = screen.getByRole("button", {
          name: "Save Settings",
        });
        expect(saveButton).toBeEnabled();
      });
    });
  });

  describe("agent prompts section", () => {
    it("shows Agent Prompts in sidebar", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const agentPromptsButton = screen.getByRole("button", {
        name: /Agent Prompts/i,
      });
      expect(agentPromptsButton).toBeInTheDocument();
    });

    it("navigates to Agent Prompts section", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const agentPromptsButton = screen.getByRole("button", {
        name: /Agent Prompts/i,
      });
      await userEvent.click(agentPromptsButton);

      // Check that Agent Prompts header is shown
      await waitFor(() => {
        expect(
          screen.getByText("Configure prompts for specialized sub-agents"),
        ).toBeInTheDocument();
      });
    });

    it("shows agent type dropdown", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const agentPromptsButton = screen.getByRole("button", {
        name: /Agent Prompts/i,
      });
      await userEvent.click(agentPromptsButton);

      await waitFor(() => {
        // Agent types from mock should be in dropdown
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });
    });
  });
});
