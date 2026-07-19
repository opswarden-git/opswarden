import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";

export interface CatalogCapability {
  name: string;
  label: string;
  description: string;
  connection_service: string | null;
}

export interface AutomationService {
  name: string;
  label: string;
  actions: CatalogCapability[];
  reactions: CatalogCapability[];
}

interface AboutResponse {
  server: { services: AutomationService[] };
}

export interface TeamConnection {
  id: string;
  team_id: string;
  service: string;
  secret_configured: boolean;
  token_configured: boolean;
  endpoint_configured: boolean;
  created_at: string;
  updated_at: string;
  verified_at: string | null;
  last_delivery_at: string | null;
  last_error_code: string | null;
  webhook_path: string | null;
}

export interface AutomationRuleDefinition {
  name: string;
  trigger_connection_id: string;
  trigger_kind: string;
  trigger_config: Record<string, string>;
  reaction_kind: string;
  reaction_connection_id: string | null;
  reaction_config: Record<string, string>;
}

export interface AutomationRule extends AutomationRuleDefinition {
  id: string;
  team_id: string;
  enabled: boolean;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface AutomationRun {
  id: string;
  delivery_id: string;
  rule_id: string | null;
  status: string;
  incident_id: string | null;
  error_code: string | null;
  started_at: string;
  finished_at: string | null;
}

const connectionsKey = (teamId: string) => ["team-automation-connections", teamId] as const;
const rulesKey = (teamId: string) => ["team-automation-rules", teamId] as const;
const runsKey = (teamId: string) => ["team-automation-runs", teamId] as const;

async function failWithCode(response: Response, fallback: string): Promise<never> {
  const body = await response.json().catch(() => null);
  throw new Error(body?.code ?? fallback);
}

export function useAutomationCatalog(enabled = true) {
  return useQuery<AutomationService[]>({
    queryKey: ["automation-catalog"],
    queryFn: async () => {
      const response = await apiFetch("/about.json");
      if (!response.ok) return failWithCode(response, "automation_catalog_failed");
      const about = (await response.json()) as AboutResponse;
      return about.server.services;
    },
    enabled,
    staleTime: 5 * 60_000,
  });
}

export function useTeamConnections(teamId: string, enabled = true) {
  return useQuery<TeamConnection[]>({
    queryKey: connectionsKey(teamId),
    queryFn: async () => {
      const response = await apiFetch(`/api/teams/${teamId}/service-connections`);
      if (!response.ok) return failWithCode(response, "connections_failed");
      return response.json();
    },
    enabled: enabled && !!teamId,
  });
}

export function useConfigureGithubConnection(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: { webhook_signing_secret?: string; personal_token?: string }) => {
      const response = await apiFetch(`/api/teams/${teamId}/service-connections/github`, {
        method: "PUT",
        body: JSON.stringify(payload),
      });
      if (!response.ok) return failWithCode(response, "github_connection_failed");
      return response.json() as Promise<TeamConnection>;
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: connectionsKey(teamId) }),
  });
}

export function useConfigureHttpConnection(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (endpoint_url: string) => {
      const response = await apiFetch(`/api/teams/${teamId}/service-connections/http`, {
        method: "PUT",
        body: JSON.stringify({ endpoint_url }),
      });
      if (!response.ok) return failWithCode(response, "http_connection_failed");
      return response.json() as Promise<TeamConnection>;
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: connectionsKey(teamId) }),
  });
}

export function useTestTeamConnection(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (connectionId: string) => {
      const response = await apiFetch(
        `/api/teams/${teamId}/service-connections/${connectionId}/test`,
        { method: "POST" },
      );
      if (!response.ok) return failWithCode(response, "connection_test_failed");
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: connectionsKey(teamId) }),
  });
}

export function useDeleteTeamConnection(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (connectionId: string) => {
      const response = await apiFetch(`/api/teams/${teamId}/service-connections/${connectionId}`, {
        method: "DELETE",
      });
      if (!response.ok) return failWithCode(response, "connection_delete_failed");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: connectionsKey(teamId) });
      queryClient.invalidateQueries({ queryKey: rulesKey(teamId) });
    },
  });
}

export function useAutomationRules(teamId: string, enabled = true) {
  return useQuery<AutomationRule[]>({
    queryKey: rulesKey(teamId),
    queryFn: async () => {
      const response = await apiFetch(`/api/teams/${teamId}/automation-rules`);
      if (!response.ok) return failWithCode(response, "automation_rules_failed");
      return response.json();
    },
    enabled: enabled && !!teamId,
  });
}

export function useCreateAutomationRule(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (definition: AutomationRuleDefinition) => {
      const response = await apiFetch(`/api/teams/${teamId}/automation-rules`, {
        method: "POST",
        body: JSON.stringify(definition),
      });
      if (!response.ok) return failWithCode(response, "automation_rule_create_failed");
      return response.json() as Promise<AutomationRule>;
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: rulesKey(teamId) }),
  });
}

export function useUpdateAutomationRule(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      ruleId,
      ...payload
    }: Partial<AutomationRuleDefinition> & {
      ruleId: string;
      enabled?: boolean;
    }) => {
      const response = await apiFetch(`/api/teams/${teamId}/automation-rules/${ruleId}`, {
        method: "PATCH",
        body: JSON.stringify(payload),
      });
      if (!response.ok) return failWithCode(response, "automation_rule_update_failed");
      return response.json() as Promise<AutomationRule>;
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: rulesKey(teamId) }),
  });
}

export function useDeleteAutomationRule(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (ruleId: string) => {
      const response = await apiFetch(`/api/teams/${teamId}/automation-rules/${ruleId}`, {
        method: "DELETE",
      });
      if (!response.ok) return failWithCode(response, "automation_rule_delete_failed");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: rulesKey(teamId) });
      queryClient.invalidateQueries({ queryKey: runsKey(teamId) });
    },
  });
}

export function useAutomationRuns(teamId: string, enabled = true, limit = 50) {
  return useQuery<AutomationRun[]>({
    queryKey: [...runsKey(teamId), limit],
    queryFn: async () => {
      const response = await apiFetch(`/api/teams/${teamId}/automation-runs?limit=${limit}`);
      if (!response.ok) return failWithCode(response, "automation_runs_failed");
      return response.json();
    },
    enabled: enabled && !!teamId,
  });
}
