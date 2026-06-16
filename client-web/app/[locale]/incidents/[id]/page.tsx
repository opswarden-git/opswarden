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
          className="text-muted hover:text-text rounded-md bg-white/5 p-2 transition-colors hover:bg-white/10"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div>
          <h1 className="text-text text-2xl font-bold tracking-tight">{incident.title}</h1>
          <div className="mt-2 flex items-center gap-4">
            <span className="text-muted font-mono text-sm">ID: {incident.id.split("-")[0]}</span>
            <StateChip status={incident.status} />
            <SeverityChip severity={incident.severity} />
            <span className="text-muted/60 text-xs">
              Created: {new Date(incident.created_at).toLocaleString()}
            </span>
            {incident.assignee ? (
              <span className="text-gold font-mono text-xs">
                Assigned: {incident.assignee.split("-")[0]}
              </span>
            ) : (
              <span className="text-muted/50 font-mono text-xs">Unassigned</span>
            )}
            <div className="ml-auto flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-2 py-0.5">
              <Eye className="text-muted h-3 w-3" />
              <span className="text-muted font-mono text-xs">{watchers.length} watching</span>
            </div>
          </div>
        </div>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-3 gap-6">
        <div className="col-span-2 flex flex-col overflow-hidden rounded-xl border border-white/5 bg-black/40">
          <Timeline incidentId={incident.id} />
        </div>

        <div className="col-span-1 flex flex-col space-y-4">
          <div className="rounded-xl border border-white/5 bg-black/40 p-6">
            <h3 className="text-text mb-4 font-bold">Actions</h3>
            <div className="space-y-3">
              <button
                onClick={() => {
                  if (user?.id) {
                    assignIncident.mutate({ incidentId: incident.id, assigneeId: user.id });
                  }
                }}
                disabled={!user?.id || incident.assignee === user.id || assignIncident.isPending}
                className="text-text flex w-full items-center justify-center gap-2 rounded-md bg-white/5 px-4 py-2 text-sm font-bold transition-colors hover:bg-white/10 disabled:opacity-50"
              >
                <UserPlus className="h-4 w-4" />
                Assign to me
              </button>

              <button
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "acknowledged" })
                }
                disabled={incident.status !== "open" || updateStatus.isPending}
                className="bg-gold/10 hover:bg-gold/20 text-gold border-gold/20 flex w-full items-center justify-center gap-2 rounded-md border px-4 py-2 text-sm font-bold transition-colors disabled:opacity-50"
              >
                <Clock className="h-4 w-4" />
                Acknowledge
              </button>

              <button
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "escalated" })
                }
                disabled={incident.status !== "acknowledged" || updateStatus.isPending}
                className="flex w-full items-center justify-center gap-2 rounded-md border border-purple-500/20 bg-purple-500/10 px-4 py-2 text-sm font-bold text-purple-400 transition-colors hover:bg-purple-500/20 disabled:opacity-50"
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
                className="flex w-full items-center justify-center gap-2 rounded-md border border-green-500/20 bg-green-500/10 px-4 py-2 text-sm font-bold text-green-400 transition-colors hover:bg-green-500/20 disabled:opacity-50"
              >
                <CheckCircle2 className="h-4 w-4" />
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

          <div className="flex-1 rounded-xl border border-white/5 bg-black/40 p-6">
            <h3 className="text-text mb-4 font-bold">Details</h3>
            <p className="text-muted text-sm whitespace-pre-wrap">
              Title: {incident.title}
              <br />
              Severity: {incident.severity}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
