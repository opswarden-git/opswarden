import type { Incident } from "@/lib/queries/incidents";

/** Events the server pushes to the client (see WEBSOCKET_SPEC.md). */
export type WsServerEvent =
  | { type: "incident_created"; incident_id: string; severity: Incident["severity"] }
  | { type: "incident_state_changed"; incident_id: string; new_state: string; by: string }
  | { type: "incident_escalated"; incident_id: string; new_severity: string; by: string }
  | { type: "incident_assigned"; incident_id: string; assigned_to: string; by: string }
  | {
      type: "timeline_entry_added";
      incident_id: string;
      entry: { entry_id: string; content: string; author: string; at: number };
    }
  | {
      type: "timeline_entry_edited";
      incident_id: string;
      entry_id: string;
      new_content: string;
      edited_at: number;
    }
  | {
      type: "reaction_added";
      incident_id: string;
      entry_id: string;
      emoji: string;
      by: string;
    }
  | {
      type: "reaction_removed";
      incident_id: string;
      entry_id: string;
      emoji: string;
      by: string;
    }
  | {
      type: "presence_update";
      resource_id: string;
      resource_type: "incident";
      watchers: string[];
    }
  | { type: "team_presence_update"; team_id: string; online_user_ids: string[] }
  | { type: "private_message_presence"; participants: string[]; watchers: string[] }
  | { type: "private_message_typing"; from: string; to: string }
  | { type: "user_typing"; incident_id: string; user_id: string }
  | {
      type: "cursor_update";
      incident_id: string;
      user_id: string;
      x: number;
      y: number;
    }
  | {
      type: "rule_triggered";
      service: string;
      rule_name: string;
      result: "incident_created" | "reaction_completed";
      incident_id: string | null;
    }
  | { type: "rule_failed"; service: string; rule_name: string; error: string }
  | { type: "member_kicked"; team_id: string; member: string; by: string }
  | {
      type: "member_banned";
      team_id: string;
      member: string;
      until: number | null;
      by: string;
    }
  | {
      type: "private_message_received";
      from: string;
      to: string;
      content: string;
      at: number;
    }
  | { type: "private_message_edited"; message_id: string; from: string; to: string; at: number }
  | { type: "release_step_validated"; release_id: string; step: string; by: string }
  | { type: "release_state_changed"; release_id: string; new_state: string };
