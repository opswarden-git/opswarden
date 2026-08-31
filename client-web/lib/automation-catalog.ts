import type { AutomationService, CatalogCapability, CatalogField } from "./queries/automations";

export type CapabilityWithService = CatalogCapability & { service: string; builtIn: boolean };

export function connectableServices(catalog: AutomationService[]) {
  return catalog.filter((service) => service.connection !== null);
}

export function catalogValues(
  fields: CatalogField[],
  stored: Record<string, string> | undefined = {},
): Record<string, string> {
  return Object.fromEntries(
    fields.map((field) => [field.name, stored?.[field.name] ?? field.default_value ?? ""]),
  );
}

export function catalogPayload(fields: CatalogField[], values: Record<string, string>) {
  return Object.fromEntries(
    fields.flatMap((field) => {
      const value = values[field.name]?.trim() ?? "";
      return value ? [[field.name, value]] : [];
    }),
  );
}

export function catalogFieldsAreValid(
  fields: CatalogField[],
  values: Record<string, string>,
  preserveExisting = false,
) {
  return fields.every(
    (field) => !field.required || preserveExisting || !!values[field.name]?.trim(),
  );
}
