import type { CollaboratorCursor } from "@/lib/ws";

function collaboratorName(identity: string) {
  return identity.includes("@") ? identity.slice(0, identity.indexOf("@")) : identity;
}

export function CollaboratorCursors({
  cursors,
  people,
}: {
  cursors: CollaboratorCursor[];
  people: Record<string, string>;
}) {
  return (
    <div className="pointer-events-none absolute inset-0 z-30 overflow-hidden" aria-hidden="true">
      {cursors.map((cursor) => {
        const name = collaboratorName(people[cursor.userId] ?? cursor.userId.slice(0, 8));
        const labelOnLeft = cursor.x > 0.78;

        return (
          <div
            key={cursor.userId}
            data-collaborator-cursor={cursor.userId}
            className="absolute transition-[left,top] duration-75 ease-linear"
            style={{ left: `${cursor.x * 100}%`, top: `${cursor.y * 100}%` }}
          >
            <svg width="20" height="24" viewBox="0 0 20 24" className="drop-shadow-sm">
              <path
                d="M2 1.5v17.2l4.35-4.1 3.05 7.15 3.25-1.4-3.05-7.1h6.15L2 1.5Z"
                fill="var(--gold)"
                stroke="white"
                strokeLinejoin="round"
                strokeWidth="1.4"
              />
            </svg>
            <span
              className="text-gold-ink bg-gold absolute top-4 max-w-36 truncate rounded px-1.5 py-0.5 text-[10px] leading-4 font-semibold shadow-sm"
              style={{
                left: labelOnLeft ? undefined : 14,
                right: labelOnLeft ? 2 : undefined,
              }}
            >
              {name}
            </span>
          </div>
        );
      })}
    </div>
  );
}
