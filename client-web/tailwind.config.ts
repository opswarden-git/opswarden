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
      /**
       * The documented radius vocabulary, so a utility cannot mean something
       * the contract does not. `DEFAULT` points at `md` the way Primer's does:
       * a nameless radius is the ordinary one, not a fifth value. `xl` and
       * `2xl` stay on Tailwind's own scale — they exist only for the
       * conversation bubble, which the design system names as its exception.
       */
      borderRadius: {
        sm: "var(--ow-radius-sm)",
        DEFAULT: "var(--ow-radius-md)",
        md: "var(--ow-radius-md)",
        lg: "var(--ow-radius-lg)",
        full: "var(--ow-radius-full)",
      },
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
          muted: "var(--ow-border-muted)",
          emphasis: "var(--ow-border-emphasis)",
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
        action: {
          primary: {
            DEFAULT: "var(--action-primary)",
            hover: "var(--action-primary-hover)",
            ink: "var(--action-primary-ink)",
          },
          secondary: {
            DEFAULT: "var(--action-secondary)",
            hover: "var(--action-secondary-hover)",
            border: "var(--action-secondary-border)",
            ink: "var(--action-secondary-ink)",
          },
          danger: {
            DEFAULT: "var(--action-danger)",
            hover: "var(--action-danger-hover)",
            ink: "var(--action-danger-ink)",
          },
        },
        feedback: {
          success: "var(--feedback-success)",
          warning: "var(--feedback-warning)",
          danger: "var(--feedback-danger)",
        },
        status: {
          neutral: "var(--status-neutral)",
          info: "var(--status-info)",
          warning: "var(--status-warning)",
          danger: "var(--status-danger)",
          success: "var(--status-success)",
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
        rel: {
          created: "var(--rel-created)",
          progress: "var(--rel-progress)",
          blocked: "var(--rel-blocked)",
          completed: "var(--rel-completed)",
          cancelled: "var(--rel-cancelled)",
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
        "sheet-content-show": "sheet-content-show 0.3s cubic-bezier(0.16, 1, 0.3, 1)",
        "sheet-content-hide": "sheet-content-hide 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
      },
    },
  },
  plugins: [],
} satisfies Config;

export default config;
