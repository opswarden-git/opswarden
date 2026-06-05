/** Tailwind-aware Stylelint config (CSS lands in Phase 1). */
export default {
  extends: ["stylelint-config-standard"],
  rules: {
    // Allow Tailwind's at-rules.
    "at-rule-no-unknown": [
      true,
      {
        ignoreAtRules: [
          "tailwind",
          "apply",
          "layer",
          "config",
          "screen",
          "variants",
          "responsive",
        ],
      },
    ],
    "selector-class-pattern": null,
    "no-descending-specificity": null,
  },
};
