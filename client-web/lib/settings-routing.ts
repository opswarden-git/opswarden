export const SETTINGS_VIEWS = ["general", "connectors"] as const;

export type SettingsView = (typeof SETTINGS_VIEWS)[number];

/** Keep unknown and legacy settings URLs on the safe account view. */
export function settingsView(value: string | null | undefined): SettingsView {
  return SETTINGS_VIEWS.includes(value as SettingsView) ? (value as SettingsView) : "general";
}
