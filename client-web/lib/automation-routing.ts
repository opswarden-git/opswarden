export const AUTOMATION_VIEWS = ["rules", "connections", "runs"] as const;

export type AutomationView = (typeof AUTOMATION_VIEWS)[number];

export function automationView(value: string | null | undefined): AutomationView {
  return AUTOMATION_VIEWS.includes(value as AutomationView) ? (value as AutomationView) : "rules";
}

/** Build the public webhook URL from the API/WS deployment hints available to the browser. */
export function automationWebhookUrl(path: string, browserOrigin: string) {
  const apiUrl = process.env.NEXT_PUBLIC_API_URL;
  if (apiUrl) return new URL(path, apiUrl).toString();

  const websocketUrl = process.env.NEXT_PUBLIC_WS_URL;
  if (websocketUrl) {
    const url = new URL(websocketUrl, browserOrigin);
    url.protocol = url.protocol === "wss:" ? "https:" : "http:";
    url.pathname = path;
    url.search = "";
    url.hash = "";
    return url.toString();
  }

  return new URL(path, browserOrigin).toString();
}
