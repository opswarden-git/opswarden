"use client";

import { ChevronLeft, ChevronRight, Circle, History, Rocket, ShieldAlert } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "@/i18n/routing";
import { cn } from "@/lib/utils";

export interface OperationsCalendarEvent {
  id: string;
  occurredAt: string;
  endedAt?: string | null;
  href: string;
  title: string;
  type: "incident" | "release" | "run";
}

type CalendarView = "month" | "week";

function startOfMonth(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

function startOfWeek(date: Date) {
  const result = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  result.setDate(result.getDate() - ((result.getDay() + 6) % 7));
  result.setHours(0, 0, 0, 0);
  return result;
}

function dayKey(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function addDays(date: Date, count: number) {
  const result = new Date(date);
  result.setDate(result.getDate() + count);
  return result;
}

function calendarDays(month: Date) {
  const gridStart = startOfWeek(startOfMonth(month));
  return Array.from({ length: 42 }, (_, index) => addDays(gridStart, index));
}

const eventIcons = {
  incident: ShieldAlert,
  release: Rocket,
  run: History,
};

const eventClassName = "border-border bg-panel-2 text-text hover:bg-panel border";

const visibleEventLimit = 2;
const hourHeight = 60;

interface CalendarLabels {
  calendar: string;
  incident: string;
  less: string;
  month: string;
  more: (count: number) => string;
  nextMonth: string;
  nextWeek: string;
  previousMonth: string;
  previousWeek: string;
  release: string;
  run: string;
  today: string;
  week: string;
}

function CalendarEventLink({
  event,
  labels,
  timeFormatter,
  week = false,
}: {
  event: OperationsCalendarEvent;
  labels: CalendarLabels;
  timeFormatter: Intl.DateTimeFormat;
  week?: boolean;
}) {
  const eventDate = new Date(event.occurredAt);
  const typeLabel = labels[event.type];
  const EventIcon = eventIcons[event.type];

  return (
    <Link
      href={event.href}
      title={`${typeLabel} · ${timeFormatter.format(eventDate)} · ${event.title}`}
      aria-label={`${typeLabel}: ${event.title}`}
      className={cn(
        "min-w-0 rounded px-1.5 text-[11px] leading-4 font-medium transition-colors",
        week ? "absolute right-1 left-1 z-10 overflow-hidden py-1" : "flex items-center gap-1 py-1",
        eventClassName,
      )}
    >
      <EventIcon
        className="inline-block h-3 w-3 shrink-0 align-[-1px]"
        strokeWidth={1.8}
        aria-hidden="true"
      />
      <time className="shrink-0 tabular-nums" dateTime={event.occurredAt}>
        {timeFormatter.format(eventDate)}
      </time>
      <span className={cn(week ? "mt-0.5 block truncate" : "truncate")}>{event.title}</span>
    </Link>
  );
}

export function OperationsCalendar({
  className,
  events,
  labels,
  locale,
}: {
  className?: string;
  events: OperationsCalendarEvent[];
  labels: CalendarLabels;
  locale: string;
}) {
  const [anchorDate, setAnchorDate] = useState(() => new Date());
  const [view, setView] = useState<CalendarView>("month");
  const [selectedTypes, setSelectedTypes] = useState<Set<OperationsCalendarEvent["type"]>>(
    () => new Set(),
  );
  const [expandedDay, setExpandedDay] = useState<string | null>(null);
  const weekScroller = useRef<HTMLDivElement>(null);
  const today = new Date();
  const currentHour = today.getHours();
  const todayKey = dayKey(today);
  const visibleMonth = startOfMonth(anchorDate);
  const weekStart = startOfWeek(anchorDate);
  const anchorTimestamp = anchorDate.getTime();
  const monthDays = calendarDays(visibleMonth);
  const weekDays = Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));

  function toggleTypeFilter(type: OperationsCalendarEvent["type"]) {
    setSelectedTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
        if (next.size === 3) {
          next.clear();
        }
      }
      return next;
    });
  }

  const filteredEvents = useMemo(() => {
    if (selectedTypes.size === 0) return events;
    return events.filter((event) => selectedTypes.has(event.type));
  }, [events, selectedTypes]);

  const eventsByDay = useMemo(() => {
    const grouped = new Map<string, OperationsCalendarEvent[]>();
    for (const event of filteredEvents) {
      const date = new Date(event.occurredAt);
      if (Number.isNaN(date.getTime())) continue;
      const key = dayKey(date);
      grouped.set(key, [...(grouped.get(key) ?? []), event]);
    }
    for (const dayEvents of grouped.values()) {
      dayEvents.sort(
        (left, right) => new Date(left.occurredAt).getTime() - new Date(right.occurredAt).getTime(),
      );
    }
    return grouped;
  }, [filteredEvents]);

  const monthLabel = new Intl.DateTimeFormat(locale, { month: "long", year: "numeric" }).format(
    visibleMonth,
  );
  const weekLabel = (() => {
    const end = addDays(weekStart, 6);
    const startLabel = new Intl.DateTimeFormat(locale, { day: "numeric", month: "short" }).format(
      weekStart,
    );
    const endLabel = new Intl.DateTimeFormat(locale, {
      day: "numeric",
      month: "short",
      year: "numeric",
    }).format(end);
    return `${startLabel} – ${endLabel}`;
  })();
  const fullDateFormatter = new Intl.DateTimeFormat(locale, { dateStyle: "full" });
  const timeFormatter = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" });
  const weekdayFormatter = new Intl.DateTimeFormat(locale, { weekday: "short" });
  const weekdays = Array.from({ length: 7 }, (_, index) =>
    weekdayFormatter.format(new Date(2024, 0, 1 + index)),
  );
  const showingToday =
    view === "month"
      ? anchorDate.getFullYear() === today.getFullYear() &&
        anchorDate.getMonth() === today.getMonth()
      : dayKey(weekStart) === dayKey(startOfWeek(today));

  useEffect(() => {
    if (view !== "week" || !weekScroller.current) return;
    const focusHour = showingToday ? currentHour : 8;
    weekScroller.current.scrollTop = Math.max(0, focusHour * hourHeight - 120);
  }, [view, showingToday, anchorTimestamp, currentHour]);

  function movePeriod(offset: number) {
    setExpandedDay(null);
    setAnchorDate((date) =>
      view === "month"
        ? new Date(date.getFullYear(), date.getMonth() + offset, 1)
        : addDays(date, offset * 7),
    );
  }

  function selectView(nextView: CalendarView) {
    setExpandedDay(null);
    setView(nextView);
  }

  return (
    <section
      aria-label={labels.calendar}
      className={cn("surface flex min-h-0 flex-col overflow-hidden rounded-md", className)}
    >
      <header className="border-border flex shrink-0 flex-wrap items-center gap-3 border-b px-4 py-2">
        <h2 className="text-text min-w-48 flex-1 text-base font-semibold capitalize">
          {view === "month" ? monthLabel : weekLabel}
        </h2>
        <div className="flex" aria-label={labels.calendar}>
          {(["week", "month"] as const).map((option) => (
            <button
              key={option}
              type="button"
              aria-pressed={view === option}
              className={cn(
                "px-2 py-1 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-current",
                view === option ? "text-gold font-semibold" : "text-muted hover:text-gold",
              )}
              onClick={() => selectView(option)}
            >
              {labels[option]}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            aria-label={view === "month" ? labels.previousMonth : labels.previousWeek}
            className="text-muted hover:text-gold inline-flex h-7 w-7 items-center justify-center rounded transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-current"
            onClick={() => movePeriod(-1)}
          >
            <ChevronLeft className="h-4 w-4" aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label={labels.today}
            className="text-muted hover:text-gold inline-flex h-7 w-7 items-center justify-center rounded transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-current"
            onClick={() => setAnchorDate(new Date())}
          >
            <Circle className="h-2.5 w-2.5 fill-current" aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label={view === "month" ? labels.nextMonth : labels.nextWeek}
            className="text-muted hover:text-gold inline-flex h-7 w-7 items-center justify-center rounded transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-current"
            onClick={() => movePeriod(1)}
          >
            <ChevronRight className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>
      </header>

      {view === "month" ? (
        <div className="min-h-0 overflow-x-auto md:flex-1">
          <div
            className="min-w-[760px] md:flex md:h-full md:flex-col"
            role="grid"
            aria-label={`${labels.calendar} — ${monthLabel}`}
          >
            <div className="border-border grid grid-cols-7 border-b" role="row">
              {weekdays.map((weekday) => (
                <div
                  key={weekday}
                  className="text-muted px-2 py-2 text-center text-[11px] font-semibold uppercase"
                  role="columnheader"
                >
                  {weekday}
                </div>
              ))}
            </div>
            <div className="grid grid-cols-7 md:min-h-0 md:flex-1 md:grid-rows-6" role="rowgroup">
              {monthDays.map((date) => {
                const key = dayKey(date);
                const dayEvents = eventsByDay.get(key) ?? [];
                const isExpanded = expandedDay === key;
                const visibleEvents = isExpanded
                  ? dayEvents
                  : dayEvents.slice(0, visibleEventLimit);
                const hiddenEventCount = dayEvents.length - visibleEvents.length;
                const belongsToMonth = date.getMonth() === visibleMonth.getMonth();
                return (
                  <div
                    key={key}
                    role="gridcell"
                    aria-label={fullDateFormatter.format(date)}
                    className={cn(
                      "border-border min-h-28 border-r border-b p-1.5 last:border-r-0 md:min-h-0",
                      isExpanded ? "md:overflow-y-auto" : "md:overflow-hidden",
                      !belongsToMonth && "bg-bg/30",
                    )}
                  >
                    <div className="mb-1 flex h-6 items-center justify-end">
                      <time
                        dateTime={key}
                        className={cn(
                          "text-muted inline-flex h-6 min-w-6 items-center justify-center rounded-full px-1.5 text-xs tabular-nums",
                          !belongsToMonth && "opacity-45",
                          key === todayKey && "text-gold font-semibold opacity-100",
                        )}
                      >
                        {date.getDate()}
                      </time>
                    </div>
                    <div className="space-y-1">
                      {visibleEvents.map((event) => (
                        <CalendarEventLink
                          key={`${event.type}-${event.id}`}
                          event={event}
                          labels={labels}
                          timeFormatter={timeFormatter}
                        />
                      ))}
                      {hiddenEventCount > 0 ? (
                        <button
                          type="button"
                          className="text-muted hover:text-text block w-full truncate px-1.5 text-left text-[11px] leading-4 font-medium transition-colors"
                          onClick={() => setExpandedDay(key)}
                        >
                          {labels.more(hiddenEventCount)}
                        </button>
                      ) : isExpanded && dayEvents.length > visibleEventLimit ? (
                        <button
                          type="button"
                          className="text-muted hover:text-text block w-full px-1.5 text-left text-[11px] leading-4 font-medium transition-colors"
                          onClick={() => setExpandedDay(null)}
                        >
                          {labels.less}
                        </button>
                      ) : null}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      ) : (
        <div className="min-h-0 overflow-x-auto md:flex-1">
          <div className="min-w-[840px] md:flex md:h-full md:flex-col">
            <div className="border-border grid shrink-0 grid-cols-[4rem_repeat(7,minmax(0,1fr))] border-b">
              <div />
              {weekDays.map((date) => (
                <div key={dayKey(date)} className="border-border border-l px-2 py-2 text-center">
                  <span className="text-muted block text-[11px] font-semibold uppercase">
                    {weekdayFormatter.format(date)}
                  </span>
                  <time
                    dateTime={dayKey(date)}
                    className={cn(
                      "mt-1 inline-flex h-7 min-w-7 items-center justify-center rounded-full px-1.5 text-sm tabular-nums",
                      dayKey(date) === todayKey ? "text-gold font-semibold" : "text-text",
                    )}
                  >
                    {date.getDate()}
                  </time>
                </div>
              ))}
            </div>
            <div
              ref={weekScroller}
              className="max-h-[640px] overflow-y-auto md:max-h-none md:min-h-0 md:flex-1"
            >
              <div
                className="grid grid-cols-[4rem_repeat(7,minmax(0,1fr))]"
                style={{ height: 24 * hourHeight }}
              >
                <div className="relative">
                  {Array.from({ length: 24 }, (_, hour) => (
                    <time
                      key={hour}
                      className="text-muted absolute right-2 -translate-y-2 text-[10px] tabular-nums"
                      style={{ top: hour * hourHeight }}
                    >
                      {String(hour).padStart(2, "0")}:00
                    </time>
                  ))}
                </div>
                {weekDays.map((date) => {
                  const key = dayKey(date);
                  const dayEvents = eventsByDay.get(key) ?? [];
                  const isCurrentDay = key === todayKey;
                  const currentMinute = today.getHours() * 60 + today.getMinutes();
                  return (
                    <div
                      key={key}
                      className="border-border relative border-l"
                      style={{
                        backgroundImage:
                          "repeating-linear-gradient(to bottom, transparent 0, transparent 59px, var(--ow-border) 59px, var(--ow-border) 60px)",
                      }}
                    >
                      {isCurrentDay ? (
                        <span
                          className="bg-status-danger pointer-events-none absolute right-0 left-0 z-20 h-px"
                          style={{ top: currentMinute }}
                          aria-hidden="true"
                        >
                          <span className="bg-status-danger absolute -top-1 -left-1 h-2 w-2 rounded-full" />
                        </span>
                      ) : null}
                      {dayEvents.map((event) => {
                        const start = new Date(event.occurredAt);
                        const end = event.endedAt ? new Date(event.endedAt) : null;
                        const top = start.getHours() * 60 + start.getMinutes();
                        const duration =
                          end && !Number.isNaN(end.getTime())
                            ? Math.max(
                                24,
                                Math.min(24 * 60 - top, (end.getTime() - start.getTime()) / 60_000),
                              )
                            : 28;
                        return (
                          <span key={`${event.type}-${event.id}`} style={{ display: "contents" }}>
                            <span
                              style={{ top, height: duration }}
                              className="absolute right-0 left-0"
                            >
                              <CalendarEventLink
                                event={event}
                                labels={labels}
                                timeFormatter={timeFormatter}
                                week
                              />
                            </span>
                          </span>
                        );
                      })}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </div>
      )}

      <footer className="border-border flex shrink-0 flex-wrap items-center gap-x-4 gap-y-1.5 border-t px-4 py-2">
        {(["incident", "release", "run"] as const).map((type) => {
          const EventIcon = eventIcons[type];
          const isSelected = selectedTypes.has(type);

          return (
            <button
              key={type}
              type="button"
              aria-pressed={isSelected}
              className={cn(
                "inline-flex items-center gap-1.5 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-current",
                isSelected ? "text-gold font-semibold" : "text-muted hover:text-gold",
              )}
              onClick={() => toggleTypeFilter(type)}
            >
              <EventIcon className="h-3.5 w-3.5 shrink-0" strokeWidth={1.8} aria-hidden="true" />
              <span>{labels[type]}</span>
            </button>
          );
        })}
      </footer>
    </section>
  );
}
