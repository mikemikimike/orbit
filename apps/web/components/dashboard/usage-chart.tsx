"use client";

import { useMemo } from "react";

/** Fill the trailing window so quiet days show as gaps instead of disappearing. */
function lastDays(days: Array<{ day: string; executions: number }>, count: number) {
  const byDay = new Map(days.map((entry) => [entry.day, entry.executions]));
  const result: Array<{ day: string; executions: number; date: Date }> = [];
  const today = new Date();
  for (let offset = count - 1; offset >= 0; offset -= 1) {
    const date = new Date(
      Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate() - offset),
    );
    const key = date.toISOString().slice(0, 10);
    result.push({ day: key, executions: byDay.get(key) ?? 0, date });
  }
  return result;
}

const dayLabel = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  timeZone: "UTC",
});

export function UsageChart({
  days,
  window = 14,
}: {
  days: Array<{ day: string; executions: number }>;
  window?: number;
}) {
  const series = useMemo(() => lastDays(days, window), [days, window]);
  const total = series.reduce((sum, entry) => sum + entry.executions, 0);
  const max = Math.max(1, ...series.map((entry) => entry.executions));

  return (
    <figure className="space-y-3">
      <figcaption className="flex items-baseline justify-between gap-3">
        <h2 className="text-[15px] font-semibold">Execution volume</h2>
        <span className="tabular text-[13px] text-muted-foreground">
          {total} in {window} days
        </span>
      </figcaption>
      <div
        role="img"
        aria-label={`${total} executions over the last ${window} days, peak ${max} in a day`}
        className="relative"
      >
        <div className="flex h-28 items-end gap-[3px] border-b border-border-strong/60">
          {series.map((entry) => (
            <div
              key={entry.day}
              className="group relative flex h-full flex-1 items-end"
              title={`${dayLabel.format(entry.date)}: ${entry.executions} execution${entry.executions === 1 ? "" : "s"}`}
            >
              <div
                className={
                  entry.executions > 0
                    ? "w-full rounded-t-[3px] bg-primary/75 transition-colors group-hover:bg-primary"
                    : "h-[3px] w-full rounded-t-[2px] bg-muted-foreground/20"
                }
                style={
                  entry.executions > 0
                    ? { height: `${Math.max(8, (entry.executions / max) * 100)}%` }
                    : undefined
                }
              />
            </div>
          ))}
        </div>
        <span className="tabular absolute right-0 top-0 -translate-y-1/2 bg-card pl-1 text-xs text-muted-foreground">
          {max}
        </span>
      </div>
      <div className="tabular flex justify-between text-xs text-muted-foreground" aria-hidden>
        <span>{dayLabel.format(series[0].date)}</span>
        <span>Today</span>
      </div>
    </figure>
  );
}
