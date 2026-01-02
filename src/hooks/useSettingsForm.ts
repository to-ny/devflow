import { useState, useCallback, useMemo, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ConfigChangedPayload,
  NotificationAction,
  ProjectConfig,
  ProviderInfo,
} from "../types/config";

export interface AgentTypeInfo {
  id: string;
  name: string;
  description: string;
}

export interface FormValidationErrors {
  model?: string;
  api_key_env?: string;
  max_tokens?: string;
  timeout_secs?: string;
  max_tool_iterations?: string;
  max_agent_depth?: string;
  search_max_results?: string;
}

export interface UseSettingsFormOptions {
  projectPath: string | null;
}

export interface UseSettingsFormReturn {
  // Loading state
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;
  successMessage: string | null;
  validationErrors: FormValidationErrors;

  // Data
  providers: ProviderInfo[];
  defaultSystemPrompt: string;
  defaultExtractionPrompt: string;
  defaultToolDescriptions: Record<string, string>;
  defaultAgentPrompts: Record<string, string>;
  agentTypes: AgentTypeInfo[];
  projectConfig: ProjectConfig | null;
  agentsMd: string | null;

  // Computed
  isDirty: boolean;
  currentProvider: ProviderInfo | undefined;
  currentModels: string[];

  // Actions
  handleSave: () => Promise<void>;
  updateAgent: (field: string, value: string | number | null) => void;
  updateExecution: (field: string, value: number) => void;
  updateSearch: (field: string, value: string | number) => void;
  updateNotifications: (
    type: "on_complete" | "on_error",
    action: NotificationAction,
    checked: boolean,
  ) => void;
  updateSystemPrompt: (value: string) => void;
  resetSystemPrompt: () => void;
  updateExtractionPrompt: (value: string) => void;
  resetExtractionPrompt: () => void;
  updatePrePostPrompt: (field: "pre" | "post", value: string) => void;
  resetPrePostPrompt: (field: "pre" | "post") => void;
  updateToolDescription: (tool: string, value: string) => void;
  resetToolDescription: (tool: string) => void;
  updateAgentPrompt: (agentId: string, value: string) => void;
  resetAgentPrompt: (agentId: string) => void;
  handleProviderChange: (providerId: string) => void;
  setAgentsMd: (value: string | null) => void;
}

export function useSettingsForm({
  projectPath,
}: UseSettingsFormOptions): UseSettingsFormReturn {
  // Loading state
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [validationErrors, setValidationErrors] =
    useState<FormValidationErrors>({});

  // Data state
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [defaultSystemPrompt, setDefaultSystemPrompt] = useState("");
  const [defaultExtractionPrompt, setDefaultExtractionPrompt] = useState("");
  const [defaultToolDescriptions, setDefaultToolDescriptions] = useState<
    Record<string, string>
  >({});
  const [defaultAgentPrompts, setDefaultAgentPrompts] = useState<
    Record<string, string>
  >({});
  const [agentTypes, setAgentTypes] = useState<AgentTypeInfo[]>([]);

  // Project config
  const [projectConfig, setProjectConfig] = useState<ProjectConfig | null>(
    null,
  );
  const [savedProjectConfig, setSavedProjectConfig] =
    useState<ProjectConfig | null>(null);

  // AGENTS.md
  const [agentsMd, setAgentsMd] = useState<string | null>(null);
  const [savedAgentsMd, setSavedAgentsMd] = useState<string | null>(null);

  // Dirty tracking
  const isDirty = useMemo(() => {
    if (!projectConfig || !savedProjectConfig) return false;
    const configDirty =
      JSON.stringify(projectConfig) !== JSON.stringify(savedProjectConfig);
    const agentsMdDirty = agentsMd !== savedAgentsMd;
    return configDirty || agentsMdDirty;
  }, [projectConfig, savedProjectConfig, agentsMd, savedAgentsMd]);

  // Load initial data
  useEffect(() => {
    const loadData = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const [
          providersData,
          systemPrompt,
          extractionPrompt,
          toolDescriptions,
          agentPrompts,
          agentTypesData,
        ] = await Promise.all([
          invoke<ProviderInfo[]>("config_get_providers"),
          invoke<string>("config_get_default_system_prompt"),
          invoke<string>("config_get_default_extraction_prompt"),
          invoke<Record<string, string>>("config_get_tool_descriptions"),
          invoke<Record<string, string>>("config_get_agent_prompts"),
          invoke<AgentTypeInfo[]>("config_get_agent_types"),
        ]);

        setProviders(providersData);
        setDefaultSystemPrompt(systemPrompt);
        setDefaultExtractionPrompt(extractionPrompt);
        setDefaultToolDescriptions(toolDescriptions);
        setDefaultAgentPrompts(agentPrompts);
        setAgentTypes(agentTypesData);

        if (projectPath) {
          const config = await invoke<ProjectConfig>("config_load_project", {
            projectPath,
          });
          setProjectConfig(config);
          setSavedProjectConfig(config);

          const agentsMdContent = await invoke<string | null>(
            "config_load_agents_md",
            { projectPath },
          );
          setAgentsMd(agentsMdContent);
          setSavedAgentsMd(agentsMdContent);
        }
      } catch (e) {
        setError(`Failed to load settings: ${e}`);
      } finally {
        setIsLoading(false);
      }
    };

    loadData();
  }, [projectPath]);

  // Listen for external config changes
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ConfigChangedPayload>("config-changed", (event) => {
      if (event.payload.project_path === projectPath) {
        invoke<ProjectConfig>("config_load_project", { projectPath }).then(
          (config) => {
            setProjectConfig(config);
            setSavedProjectConfig(config);
          },
        );
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [projectPath]);

  // Provider handling
  const currentProvider = useMemo(
    () => providers.find((p) => p.id === projectConfig?.agent.provider),
    [providers, projectConfig],
  );

  const currentModels = useMemo(
    () => currentProvider?.models || [],
    [currentProvider],
  );

  // Validation
  const validateForm = useCallback((): FormValidationErrors => {
    const errors: FormValidationErrors = {};
    if (!projectConfig) return errors;

    const { agent, execution, search } = projectConfig;

    if (!agent.model?.trim()) {
      errors.model = "Model is required";
    }
    if (!agent.api_key_env?.trim()) {
      errors.api_key_env = "API key environment variable is required";
    }
    if (agent.max_tokens < 1 || agent.max_tokens > 200000) {
      errors.max_tokens = "Max tokens must be between 1 and 200,000";
    }
    if (execution.timeout_secs < 1 || execution.timeout_secs > 600) {
      errors.timeout_secs = "Timeout must be between 1 and 600 seconds";
    }
    if (
      execution.max_tool_iterations < 1 ||
      execution.max_tool_iterations > 1000
    ) {
      errors.max_tool_iterations = "Max iterations must be between 1 and 1,000";
    }
    if (execution.max_agent_depth < 1 || execution.max_agent_depth > 10) {
      errors.max_agent_depth = "Max agent depth must be between 1 and 10";
    }
    if (search.max_results < 1 || search.max_results > 50) {
      errors.search_max_results = "Max results must be between 1 and 50";
    }

    return errors;
  }, [projectConfig]);

  // Update handlers
  const updateAgent = useCallback(
    (field: string, value: string | number | null) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        return { ...prev, agent: { ...prev.agent, [field]: value } };
      });
      setSuccessMessage(null);
      setValidationErrors((prev) => ({ ...prev, [field]: undefined }));
    },
    [],
  );

  const updateExecution = useCallback((field: string, value: number) => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      return { ...prev, execution: { ...prev.execution, [field]: value } };
    });
    setSuccessMessage(null);
    setValidationErrors((prev) => ({ ...prev, [field]: undefined }));
  }, []);

  const updateSearch = useCallback((field: string, value: string | number) => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      return { ...prev, search: { ...prev.search, [field]: value } };
    });
    setSuccessMessage(null);
    setValidationErrors((prev) => ({ ...prev, [field]: undefined }));
  }, []);

  const updateNotifications = useCallback(
    (
      type: "on_complete" | "on_error",
      action: NotificationAction,
      checked: boolean,
    ) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        const actions = [...(prev.notifications[type] || [])];
        if (checked && !actions.includes(action)) {
          actions.push(action);
        } else if (!checked) {
          const idx = actions.indexOf(action);
          if (idx > -1) actions.splice(idx, 1);
        }
        return {
          ...prev,
          notifications: { ...prev.notifications, [type]: actions },
        };
      });
      setSuccessMessage(null);
    },
    [],
  );

  const updateSystemPrompt = useCallback(
    (value: string) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        const currentEffectiveValue = prev.system_prompt ?? defaultSystemPrompt;
        if (value === currentEffectiveValue) return prev;
        if (value === defaultSystemPrompt) {
          return { ...prev, system_prompt: null };
        }
        return { ...prev, system_prompt: value };
      });
      setSuccessMessage(null);
    },
    [defaultSystemPrompt],
  );

  const resetSystemPrompt = useCallback(() => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      return { ...prev, system_prompt: null };
    });
    setSuccessMessage(null);
  }, []);

  const updateExtractionPrompt = useCallback(
    (value: string) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        const currentEffectiveValue =
          prev.extraction_prompt ?? defaultExtractionPrompt;
        if (value === currentEffectiveValue) return prev;
        if (value === defaultExtractionPrompt) {
          return { ...prev, extraction_prompt: null };
        }
        return { ...prev, extraction_prompt: value };
      });
      setSuccessMessage(null);
    },
    [defaultExtractionPrompt],
  );

  const resetExtractionPrompt = useCallback(() => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      return { ...prev, extraction_prompt: null };
    });
    setSuccessMessage(null);
  }, []);

  const updatePrePostPrompt = useCallback(
    (field: "pre" | "post", value: string) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        if (value === prev.prompts[field]) return prev;
        return { ...prev, prompts: { ...prev.prompts, [field]: value } };
      });
      setSuccessMessage(null);
    },
    [],
  );

  const resetPrePostPrompt = useCallback((field: "pre" | "post") => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      return { ...prev, prompts: { ...prev.prompts, [field]: "" } };
    });
    setSuccessMessage(null);
  }, []);

  const updateToolDescription = useCallback(
    (tool: string, value: string) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        const defaultDesc = defaultToolDescriptions[tool];
        const currentEffectiveValue =
          prev.tool_descriptions?.[tool] ?? defaultDesc;

        if (value === currentEffectiveValue) return prev;

        if (value === defaultDesc) {
          if (!prev.tool_descriptions) return prev;
          const updated = { ...prev.tool_descriptions };
          delete updated[tool];
          return {
            ...prev,
            tool_descriptions: Object.keys(updated).length > 0 ? updated : null,
          };
        }

        const current = prev.tool_descriptions || {};
        return { ...prev, tool_descriptions: { ...current, [tool]: value } };
      });
      setSuccessMessage(null);
    },
    [defaultToolDescriptions],
  );

  const resetToolDescription = useCallback((tool: string) => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      if (!prev.tool_descriptions) return prev;
      const updated = { ...prev.tool_descriptions };
      delete updated[tool];
      return {
        ...prev,
        tool_descriptions: Object.keys(updated).length > 0 ? updated : null,
      };
    });
    setSuccessMessage(null);
  }, []);

  const updateAgentPrompt = useCallback(
    (agentId: string, value: string) => {
      setProjectConfig((prev) => {
        if (!prev) return prev;
        const defaultPrompt = defaultAgentPrompts[agentId];
        const currentEffectiveValue =
          prev.agent_prompts?.[agentId] ?? defaultPrompt;

        if (value === currentEffectiveValue) return prev;

        if (value === defaultPrompt) {
          if (!prev.agent_prompts) return prev;
          const updated = { ...prev.agent_prompts };
          delete updated[agentId];
          return {
            ...prev,
            agent_prompts: Object.keys(updated).length > 0 ? updated : null,
          };
        }

        const current = prev.agent_prompts || {};
        return { ...prev, agent_prompts: { ...current, [agentId]: value } };
      });
      setSuccessMessage(null);
    },
    [defaultAgentPrompts],
  );

  const resetAgentPrompt = useCallback((agentId: string) => {
    setProjectConfig((prev) => {
      if (!prev) return prev;
      if (!prev.agent_prompts) return prev;
      const updated = { ...prev.agent_prompts };
      delete updated[agentId];
      return {
        ...prev,
        agent_prompts: Object.keys(updated).length > 0 ? updated : null,
      };
    });
    setSuccessMessage(null);
  }, []);

  const handleProviderChange = useCallback(
    (providerId: string) => {
      const provider = providers.find((p) => p.id === providerId);
      setProjectConfig((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          agent: {
            ...prev.agent,
            provider: providerId,
            model: provider?.models[0] || "",
            api_key_env: provider?.default_api_key_env || "",
          },
        };
      });
      setSuccessMessage(null);
    },
    [providers],
  );

  const handleSave = useCallback(async () => {
    if (!projectPath || !projectConfig) return;

    const errors = validateForm();
    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      setError("Please fix the validation errors before saving");
      return;
    }

    try {
      setIsSaving(true);
      setError(null);
      setSuccessMessage(null);

      await invoke("config_save_project", {
        projectPath,
        config: projectConfig,
      });
      setSavedProjectConfig(projectConfig);

      await invoke("config_save_agents_md", { projectPath, content: agentsMd });
      setSavedAgentsMd(agentsMd);

      setSuccessMessage("Settings saved successfully");
    } catch (e) {
      setError(`Failed to save: ${e}`);
    } finally {
      setIsSaving(false);
    }
  }, [projectPath, projectConfig, agentsMd, validateForm]);

  const handleSetAgentsMd = useCallback((value: string | null) => {
    setAgentsMd(value);
    setSuccessMessage(null);
  }, []);

  return {
    isLoading,
    isSaving,
    error,
    successMessage,
    validationErrors,
    providers,
    defaultSystemPrompt,
    defaultExtractionPrompt,
    defaultToolDescriptions,
    defaultAgentPrompts,
    agentTypes,
    projectConfig,
    agentsMd,
    isDirty,
    currentProvider,
    currentModels,
    handleSave,
    updateAgent,
    updateExecution,
    updateSearch,
    updateNotifications,
    updateSystemPrompt,
    resetSystemPrompt,
    updateExtractionPrompt,
    resetExtractionPrompt,
    updatePrePostPrompt,
    resetPrePostPrompt,
    updateToolDescription,
    resetToolDescription,
    updateAgentPrompt,
    resetAgentPrompt,
    handleProviderChange,
    setAgentsMd: handleSetAgentsMd,
  };
}
