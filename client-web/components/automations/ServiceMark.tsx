import { Globe2, Webhook } from "lucide-react";
import Image from "next/image";
import { MdAlternateEmail, MdHttp } from "react-icons/md";
import type { AutomationService } from "@/lib/queries/automations";

const providerMarks: Record<string, string> = {
  alertmanager: "/assets/alertmanager.svg",
  github: "/assets/github-patched.webp",
  gitlab: "/assets/gitlab.webp",
};

/** Decorative connector mark; the surrounding row or button owns its name. */
export function ServiceMark({ inline, service }: { inline?: boolean; service: AutomationService }) {
  const box = inline ? "h-[18px] w-[18px]" : "h-7 w-7";
  const asset = providerMarks[service.name];
  if (asset) {
    const side = inline ? 18 : 28;
    return (
      <Image src={asset} alt="" width={side} height={side} className={`${box} object-contain`} />
    );
  }
  if (service.name === "email") {
    return <MdAlternateEmail className={box} aria-hidden="true" />;
  }
  if (service.name === "http") {
    return <MdHttp className={inline ? box : "h-8 w-8"} aria-hidden="true" />;
  }
  if (service.name === "generic") return <Webhook className={box} aria-hidden="true" />;
  return <Globe2 className={box} aria-hidden="true" />;
}
