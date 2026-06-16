"use client";

import React, { use, useEffect } from "react";
import { useAssignIncident, useIncident, useUpdateIncidentStatus } from "@/lib/queries/incidents";
import { useAuthStore } from "@/store/auth";
import { useWsStore } from "@/lib/ws";
import { StateChip } from "@/components/incidents/StateChip";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { Timeline } from "@/components/incidents/Timeline";
import { ArrowLeft, Clock, ShieldAlert, CheckCircle2, UserPlus, Eye } from "lucide-react";
import { Link } from "@/i18n/routing";

export default function WarRoomPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const { data: incident, isLoading, error } = useIncident(id);
  const updateStatus = useUpdateIncidentStatus();
  const assignIncident = useAssignIncident();
  const user = useAuthStore((s) => s.user);

  const sendJson = useWsStore((s) => s.sendJson);
  const watchers = useWsStore((s) => s.watchers);

  useEffect(() => {
    if (id) {
      sendJson({ type: "watch", incident_id: id });
    }
    return () => {
      if (id) {
        sendJson({ type: "unwatch", incident_id: id });
      }
    };
  }, [id, sendJson]);

  if (isLoading) {
    return <div className="text-muted animate-pulse p-10 text-center">Loading War Room...</div>;
  }

  if (error || !incident) {
    return (
      <div className="mx-auto max-w-5xl p-6 text-center">
        <p className="text-red-500">Failed to load incident details.</p>
        <Link href="/incidents" className="text-gold mt-4 inline-block hover:underline">
          Return to Incidents
        </Link>
      </div>
    );
  }

  return (
    <div className="mx-auto flex h-[calc(100vh-80px)] max-w-6xl flex-col space-y-6 p-6">
      <div className="flex items-center gap-4">
        <Link
          href="/incidents"
          className="text-muted hover:text-text rounded-md bg-panel p-2 border border-border transition-colors hover:border-gold"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div className="flex-1">
          <h1 className="text-text text-2xl font-bold tracking-tight">{incident.title}</h1>
          <div className="mt-2 flex items-center gap-4">
            <span className="text-muted font-mono text-sm">ID: {incident.id.split("-")[0]}</span>
            <StateChip status={incident.status} />
            <SeverityChip severity={incident.severity} />
            <span className="text-muted/60 text-xs">
              T-Minus: {new Date(incident.created_at).toLocaleString()}
            </span>
            {incident.assignee ? (
              <span className="text-text inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-xs font-medium">
                <UserPlus className="h-3 w-3 text-gold" />
                {incident.assignee.split("-")[0]}
              </span>
            ) : (
              <span className="text-muted/50 inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-xs font-medium">
                Unassigned
              </span>
            )}
            <div className="ml-auto flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1">
              <span className="relative flex h-2 w-2">
                {watchers.length > 0 ? (
                  <>
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                    <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                  </>
                ) : (
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-muted/50"></span>
                )}
              </span>
              <span className="text-text text-xs font-medium">{watchers.length} Watchers</span>
            </div>
          </div>
        </div>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-3 gap-6">
        <div className="col-span-2 flex flex-col overflow-hidden rounded-xl border border-white/5 bg-black/40">
          <Timeline incidentId={incident.id} />
        </div>

        <div className="col-span-1 flex flex-col space-y-4">
          <div className="rounded-xl border border-white/5 bg-white/5 p-6">
            <h3 className="text-text font-bold mb-4">Command Actions</h3>
            <div className="space-y-3">
              <button
                onClick={() => {
                  if (user?.id) {
                    assignIncident.mutate({ incidentId: incident.id, assigneeId: user.id });
                  }
                }}
                disabled={!user?.id || incident.assignee === user.id || assignIncident.isPending}
                className="bg-gold hover:bg-gold-hover text-[#1a1405] flex w-full items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-bold transition-colors disabled:opacity-50 disabled:pointer-events-none"
              >
                <UserPlus className="h-4 w-4" />
                Assume Lead
              </button>

              <button
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "acknowledged" })
                }
                disabled={incident.status !== "open" || updateStatus.isPending}
                className="text-text hover:border-st-ack/50 hover:bg-st-ack/10 flex w-full items-center justify-center gap-2 rounded-lg border border-white/10 bg-transparent px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50 disabled:pointer-events-none"
              >
                <Clock className="h-4 w-4 text-st-ack" />
                Acknowledge
              </button>

              <button
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "escalated" })
                }
                disabled={incident.status !== "acknowledged" || updateStatus.isPending}
                className="bg-red-600 hover:bg-red-700 text-white flex w-full items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-bold transition-colors disabled:opacity-50 disabled:pointer-events-none"
              >
                <ShieldAlert className="h-4 w-4" />
                Escalate
              </button>

              <button
                onClick={() => updateStatus.mutate({ incidentId: incident.id, status: "resolved" })}
                disabled={
                  incident.status === "resolved" ||
                  incident.status === "open" ||
                  updateStatus.isPending
                }
                className="text-text hover:border-st-res/50 hover:bg-st-res/10 flex w-full items-center justify-center gap-2 rounded-lg border border-white/10 bg-transparent px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50 disabled:pointer-events-none"
              >
                <CheckCircle2 className="h-4 w-4 text-st-res" />
                Resolve
              </button>
            </div>
            {updateStatus.isError && (
              <p className="mt-4 text-center text-xs text-red-500">{updateStatus.error.message}</p>
            )}
            {assignIncident.isError && (
              <p className="mt-4 text-center text-xs text-red-500">
                {assignIncident.error.message}
              </p>
            )}
          </div>

          <div className="flex-1 rounded-xl border border-white/5 bg-white/5 p-6">
            <h3 className="text-text font-bold mb-4">Payload Details</h3>
            <p className="text-muted text-sm whitespace-pre-wrap">
              <span className="text-text">Title:</span> {incident.title}
              <br /><br />
              <span className="text-text">Severity Level:</span> {incident.severity.toUpperCase()}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
