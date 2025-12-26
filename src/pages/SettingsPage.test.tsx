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
};

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
      return Promise.resolve(undefined);
    });
  });

  describe("loading state", () => {
    it("shows loading state initially", () => {
      render(<SettingsPage />);
      expect(screen.getByText("Loading settings...")).toBeInTheDocument();
    });

    it("loads and displays config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      expect(
        screen.getByRole("heading", { name: /Project Settings/i }),
      ).toBeInTheDocument();
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

      // The model select has id="model-select", not linked to label
      const modelSelect = document.getElementById(
        "model-select",
      ) as HTMLSelectElement;
      expect(modelSelect.value).toBe("claude-sonnet-4-20250514");
    });

    it("populates API key env field from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const apiKeyInput = screen.getByLabelText(
        "API Key Environment Variable",
      ) as HTMLInputElement;
      expect(apiKeyInput.value).toBe("ANTHROPIC_API_KEY");
    });

    it("populates max tokens field from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const maxTokensInput = screen.getByLabelText(
        "Max Tokens",
      ) as HTMLInputElement;
      expect(maxTokensInput.value).toBe("8192");
    });

    it("populates prompts fields from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const prePrompt = screen.getByLabelText(
        "Pre-prompt",
      ) as HTMLTextAreaElement;
      const postPrompt = screen.getByLabelText(
        "Post-prompt",
      ) as HTMLTextAreaElement;

      expect(prePrompt.value).toBe("Pre-prompt text");
      expect(postPrompt.value).toBe("Post-prompt text");
    });

    it("populates execution fields from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const timeout = screen.getByLabelText(
        "Tool Timeout (seconds)",
      ) as HTMLInputElement;
      const maxIterations = screen.getByLabelText(
        "Max Tool Iterations",
      ) as HTMLInputElement;
      const maxDepth = screen.getByLabelText(
        "Max Sub-agent Depth",
      ) as HTMLInputElement;

      expect(timeout.value).toBe("30");
      expect(maxIterations.value).toBe("50");
      expect(maxDepth.value).toBe("3");
    });

    it("populates notification checkboxes from config", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // on_complete has "sound"
      const completeSoundCheckbox = screen
        .getAllByRole("checkbox")
        .find(
          (cb) => cb.getAttribute("name") === "on_complete_sound",
        ) as HTMLInputElement;
      expect(completeSoundCheckbox.checked).toBe(true);

      // on_error has "sound" and "window"
      const errorSoundCheckbox = screen
        .getAllByRole("checkbox")
        .find(
          (cb) => cb.getAttribute("name") === "on_error_sound",
        ) as HTMLInputElement;
      const errorWindowCheckbox = screen
        .getAllByRole("checkbox")
        .find(
          (cb) => cb.getAttribute("name") === "on_error_window",
        ) as HTMLInputElement;
      expect(errorSoundCheckbox.checked).toBe(true);
      expect(errorWindowCheckbox.checked).toBe(true);
    });
  });

  describe("validation", () => {
    it("shows error when model is empty", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Select custom model option which sets model to empty string
      const modelSelect = document.getElementById(
        "model-select",
      ) as HTMLSelectElement;
      await userEvent.selectOptions(modelSelect, "__custom__");

      // Try to save
      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(screen.getByText("Model is required")).toBeInTheDocument();
      });
    });

    it("shows error when API key env is empty", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const apiKeyInput = screen.getByLabelText("API Key Environment Variable");
      await userEvent.clear(apiKeyInput);

      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(
          screen.getByText("API key environment variable is required"),
        ).toBeInTheDocument();
      });
    });

    it("shows error when max tokens is out of range", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const maxTokensInput = screen.getByLabelText("Max Tokens");
      await userEvent.clear(maxTokensInput);
      await userEvent.type(maxTokensInput, "300000");

      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(
          screen.getByText("Max tokens must be between 1 and 200,000"),
        ).toBeInTheDocument();
      });
    });

    it("shows error when timeout is out of range", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const timeoutInput = screen.getByLabelText("Tool Timeout (seconds)");
      await userEvent.clear(timeoutInput);
      await userEvent.type(timeoutInput, "1000");

      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(
          screen.getByText("Timeout must be between 1 and 600 seconds"),
        ).toBeInTheDocument();
      });
    });
  });

  describe("save functionality", () => {
    it("invokes config_save_project on save", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Make a change to enable save
      const maxTokensInput = screen.getByLabelText("Max Tokens");
      await userEvent.clear(maxTokensInput);
      await userEvent.type(maxTokensInput, "4096");

      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("config_save_project", {
          projectPath: "/test/project",
          config: expect.objectContaining({
            agent: expect.objectContaining({
              max_tokens: 4096,
            }),
          }),
        });
      });
    });

    it("shows success message after save", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Make a change
      const maxTokensInput = screen.getByLabelText("Max Tokens");
      await userEvent.clear(maxTokensInput);
      await userEvent.type(maxTokensInput, "4096");

      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(
          screen.getByText("Settings saved successfully"),
        ).toBeInTheDocument();
      });
    });

    it("disables save button when no changes", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

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

      const maxTokensInput = screen.getByLabelText("Max Tokens");
      await userEvent.clear(maxTokensInput);
      await userEvent.type(maxTokensInput, "4096");

      const saveButton = screen.getByRole("button", { name: "Save Settings" });
      expect(saveButton).not.toBeDisabled();
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

    it("shows error when save fails", async () => {
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === "config_get_providers") {
          return Promise.resolve(mockProviders);
        }
        if (cmd === "config_load_project") {
          return Promise.resolve(mockConfig);
        }
        if (cmd === "config_save_project") {
          return Promise.reject(new Error("Permission denied"));
        }
        return Promise.resolve(undefined);
      });

      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      // Make a change
      const maxTokensInput = screen.getByLabelText("Max Tokens");
      await userEvent.clear(maxTokensInput);
      await userEvent.type(maxTokensInput, "4096");

      const saveButton = screen.getByRole("button", { name: /Save/i });
      await userEvent.click(saveButton);

      await waitFor(() => {
        expect(screen.getByText(/Failed to save/)).toBeInTheDocument();
      });
    });
  });

  describe("unsaved changes indicator", () => {
    it("shows unsaved indicator when form is dirty", async () => {
      render(<SettingsPage />);

      await waitFor(() => {
        expect(
          screen.queryByText("Loading settings..."),
        ).not.toBeInTheDocument();
      });

      const maxTokensInput = screen.getByLabelText("Max Tokens");
      await userEvent.clear(maxTokensInput);
      await userEvent.type(maxTokensInput, "4096");

      // Check for unsaved indicator (has title attribute)
      const indicator = screen.getByTitle("Unsaved changes");
      expect(indicator).toBeInTheDocument();
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
});
