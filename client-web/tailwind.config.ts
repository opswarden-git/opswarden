import type { Config } from "tailwindcss";

const config = {
  darkMode: "class",
  content: [
    "./pages/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./app/**/*.{ts,tsx}",
    "./src/**/*.{ts,tsx}",
  ],
  prefix: "",
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        bg: {
          DEFAULT: "var(--bg)",
          2: "var(--bg-2)",
        },
        panel: {
          DEFAULT: "var(--panel)",
          2: "var(--panel-2)",
        },
        border: {
          DEFAULT: "var(--ow-border)",
          2: "var(--ow-border-2)",
        },
        text: "var(--text)",
        muted: {
          DEFAULT: "var(--ow-muted)",
          2: "var(--ow-muted-2)",
        },
        gold: {
          DEFAULT: "var(--gold)",
          hover: "var(--gold-hover)",
          dim: "var(--gold-dim)",
          ink: "var(--gold-ink)",
        },
        danger: {
          DEFAULT: "var(--danger)",
          hover: "var(--danger-hover)",
          ink: "var(--danger-ink)",
          text: "var(--danger-text)",
        },
        sev: {
          low: "var(--sev-low)",
          medium: "var(--sev-medium)",
          high: "var(--sev-high)",
          critical: "var(--sev-critical)",
        },
        st: {
          open: "var(--st-open)",
          ack: "var(--st-ack)",
          esc: "var(--st-esc)",
          res: "var(--st-res)",
        },
      },
      fontFamily: {
        sans: ["var(--font-sans)", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["var(--font-mono)", "ui-monospace", "SFMono-Regular", "monospace"],
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
        "dialog-overlay-show": {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        "dialog-overlay-hide": {
          from: { opacity: "1" },
          to: { opacity: "0" },
        },
        "dialog-content-show": {
          from: { opacity: "0", transform: "translate(-50%, -48%) scale(0.96)" },
          to: { opacity: "1", transform: "translate(-50%, -50%) scale(1)" },
        },
        "dialog-content-hide": {
          from: { opacity: "1", transform: "translate(-50%, -50%) scale(1)" },
          to: { opacity: "0", transform: "translate(-50%, -48%) scale(0.96)" },
        },
        "sheet-content-show": {
          from: { transform: "translateY(100%)" },
          to: { transform: "translateY(0)" },
        },
        "sheet-content-hide": {
          from: { transform: "translateY(0)" },
          to: { transform: "translateY(100%)" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
        "dialog-overlay-show": "dialog-overlay-show 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
        "dialog-overlay-hide": "dialog-overlay-hide 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
        "dialog-content-show": "dialog-content-show 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
        "dialog-content-hide": "dialog-content-hide 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
        "sheet-content-show": "sheet-content-show 0.3s cubic-bezier(0.16, 1, 0.3, 1)",
        "sheet-content-hide": "sheet-content-hide 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
      },
    },
  },
  plugins: [],
} satisfies Config;

export default config;
