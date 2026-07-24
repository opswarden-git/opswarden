import { describe, expect, it } from "vitest";
import {
  catalogFieldsAreValid,
  catalogPayload,
  catalogValues,
  connectableServices,
} from "./automation-catalog";
import type { AutomationService, CatalogField } from "./queries/automations";

const fields: CatalogField[] = [
  {
    name: "api_key",
    label: "API key",
    description: "A future provider credential",
    input_type: "password",
    required: true,
    default_value: null,
    options: [],
  },
  {
    name: "region",
    label: "Region",
    description: "Optional region",
    input_type: "select",
    required: false,
    default_value: "eu",
    options: [
      { value: "eu", label: "Europe" },
      { value: "us", label: "United States" },
    ],
  },
];

describe("server-driven automation catalog", () => {
  it("derives defaults, validation and payloads from arbitrary server fields", () => {
    expect(catalogValues(fields)).toEqual({ api_key: "", region: "eu" });
    expect(catalogFieldsAreValid(fields, { api_key: "", region: "eu" })).toBe(false);
    expect(catalogFieldsAreValid(fields, { api_key: " secret ", region: "eu" })).toBe(true);
    expect(catalogPayload(fields, { api_key: " secret ", region: "" })).toEqual({
      api_key: "secret",
    });
    expect(catalogFieldsAreValid(fields, { api_key: "" }, true)).toBe(true);
  });

  it("discovers connectable services without a client-side service list", () => {
    const futureService = {
      name: "future-service",
      label: "Future",
      actions: [],
      reactions: [],
      connection: {
        description: "Discovered from /about.json",
        fields,
        oauth: null,
        testable: true,
      },
    } satisfies AutomationService;
    const internalService = {
      ...futureService,
      name: "internal",
      connection: null,
    } satisfies AutomationService;

    expect(connectableServices([futureService, internalService])).toEqual([futureService]);
  });
});
