"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function UsageChart({
  days,
}: {
  days: Array<{ day: string; executions: number }>;
}) {
  const max = Math.max(1, ...days.map((d) => d.executions));
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle>Execution volume</CardTitle>
        <p className="text-xs text-muted-foreground">Last 14 days</p>
      </CardHeader>
      <CardContent>
        {days.length === 0 ? (
          <p className="py-8 text-center text-sm text-muted-foreground">No executions yet.</p>
        ) : (
          <div className="flex h-32 items-end gap-1.5">
            {days.map((day) => (
              <div key={day.day} className="group flex h-full flex-1 flex-col justify-end gap-1">
                <div
                  className="w-full rounded-t bg-primary/70 transition-colors group-hover:bg-primary"
                  style={{ height: `${Math.max(6, (day.executions / max) * 100)}%` }}
                  title={`${day.day}: ${day.executions} executions`}
                />
                <span className="truncate text-center text-[9px] text-muted-foreground">
                  {day.day.slice(5)}
                </span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
