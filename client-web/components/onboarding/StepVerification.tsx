import React, { useEffect, useState, useRef } from "react";
import { useRouter } from "next/navigation";

interface StepProps {
  data: any;
}

const CONSOLE_LOGS = [
  "INITIALIZING SECURITY PROTOCOLS...",
  "GENERATING CRYPTOGRAPHIC KEYPAIR...",
  "CONNECTING TO ENCRYPTED STREAM SERVER...",
  "AUTHORIZING OPERATOR CLEARANCE...",
  "STREAM ESTABLISHED: Paris-Core-NOC-1 [CONNECTED]",
  "CONFIGURING PROMETHEUS SCRAPING STREAM...",
  "RESOLVING INGESTION ENDPOINTS...",
  "SYSTEM ONLINE. PREPARING LAUNCH CONTROLLER...",
];

export function StepVerification({ data }: StepProps) {
  const router = useRouter();
  const [logs, setLogs] = useState<string[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let currentIdx = 0;
    const interval = setInterval(() => {
      if (currentIdx < CONSOLE_LOGS.length) {
        setLogs((prev) => [
          ...prev,
          `[${new Date().toLocaleTimeString()}] ${CONSOLE_LOGS[currentIdx]}`,
        ]);
        currentIdx++;
      } else {
        clearInterval(interval);
        // Delay redirection slightly so user sees system online
        setTimeout(() => {
          router.push("/");
        }, 1200);
      }
    }, 450);

    return () => clearInterval(interval);
  }, [router]);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div className="mx-auto w-full max-w-sm space-y-6">
      <div
        ref={containerRef}
        className="scrollbar-thin scrollbar-thumb-muted/10 h-64 w-full space-y-1.5 overflow-y-auto rounded-md border border-white/5 bg-black/60 p-4 font-mono text-xs text-green-500"
      >
        <div>SYSTEM BOOT LOADER v1.2.0-STABLE</div>
        <div>OPERATOR: {data.operatorName || "UNKNOWN"}</div>
        <div>STATION: {data.stationName || "UNKNOWN"}</div>
        <div className="my-2 border-t border-white/5"></div>
        {logs.map((log, i) => (
          <div key={i} className="animate-fade-in">
            {log}
          </div>
        ))}
        {logs.length < CONSOLE_LOGS.length && <div className="animate-pulse">_</div>}
      </div>
    </div>
  );
}
