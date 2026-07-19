/** Tailwind-aware Stylelint config (CSS lands in Phase 1). */
const config = {
  extends: ["stylelint-config-standard"],
  rules: {
    // Allow Tailwind's at-rules.
    "at-rule-no-unknown": [
      true,
      {
        ignoreAtRules: ["tailwind", "apply", "layer", "config", "screen", "variants", "responsive"],
      },
    ],
    "selector-class-pattern": null,
    "no-descending-specificity": null,
    // Tailwind 4 documents package imports with the string form.
    "import-notation": null,
    // Native controls still need the prefixed appearance declaration on Safari.
    "property-no-vendor-prefix": null,
  },
};

export default config;
