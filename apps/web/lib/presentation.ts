// Human-friendly labels for the technical primitives ORBIT exposes in the UI.
// Kept in one place so capability ids, effects, cron expressions and event kinds
// read like English everywhere instead of internal identifiers.

const CAPABILITY_LABELS: Record<string, string> = {
  "notes.list": "List notes",
  "notes.create": "Create note",
  "messages.list": "Read messages",
  "messages.send": "Send message",
  "issues.list": "List issues",
  "issues.get": "Read issue",
  "issues.comment": "Comment on issue",
  "issues.create": "Create issue",
  "issues.update": "Update issue",
  "pulls.list": "List pull requests",
  "pulls.get": "Read pull request",
  "pulls.create": "Open pull request",
  "checks.list": "Check CI status",
  "repo.checkout": "Check out repository",
  "repo.commit": "Commit changes",
  "repo.push": "Push branch",
  "projects.list": "List projects",
  "group_issues.list": "List group issues",
  "boards.list": "List issue boards",
  "merge_requests.list": "List merge requests",
  "merge_requests.get": "Read merge request",
  "merge_requests.create": "Open merge request",
  "merge_requests.comment": "Comment on merge request",
  "merge_requests.approve": "Approve merge request",
  "merge_requests.merge": "Merge request",
  "pipelines.status": "Check pipeline status",
  "posts.search": "Search posts",
  "posts.create": "Create post",
  "request.send": "Send webhook",
};

const EFFECT_LABELS: Record<string, { label: string; tone: "safe" | "write" | "external" }> = {
  read: { label: "Read only", tone: "safe" },
  write: { label: "Can change data", tone: "write" },
  external_side_effect: { label: "Can act externally", tone: "external" },
  none: { label: "No side effects", tone: "safe" },
};

const AUTONOMY_NAMES: Record<number, string> = {
  0: "Manual",
  1: "Observe",
  2: "Assist",
  3: "Act with approval",
  4: "Fully autonomous",
};

export function capabilityLabel(capabilityId: string | null | undefined): string {
  if (!capabilityId) return "Action";
  const exact = CAPABILITY_LABELS[capabilityId];
  if (exact) return exact;

  // Generic fallback: "merge_requests.approve" -> "Approve merge requests".
  const [resource, action] = capabilityId.split(".");
  if (resource && action) {
    const resourceWords = resource.replace(/_/g, " ");
    const actionWord = action === "list" ? "List" : action.replace(/_/g, " ");
    return `${cap(actionWord)} ${resourceWords}`.replace(/\s+/g, " ").trim();
  }
  return capabilityId.replace(/[_-]+/g, " ");
}

export function effectLabel(effect: string | null | undefined): string {
  if (!effect) return "Unknown effect";
  return EFFECT_LABELS[effect]?.label ?? effect.replace(/_/g, " ");
}

export function effectTone(
  effect: string | null | undefined,
): "safe" | "write" | "external" {
  if (!effect) return "safe";
  return EFFECT_LABELS[effect]?.tone ?? "safe";
}

export function autonomyName(level: number | null | undefined): string {
  if (level === null || level === undefined) return "Autonomy";
  return AUTONOMY_NAMES[level] ?? `Level ${level}`;
}

export function eventKindLabel(kind: string | null | undefined): string {
  switch (kind) {
    case "log":
      return "Log";
    case "status":
      return "Status";
    case "error":
      return "Error";
    case "output":
      return "Output";
    case "tool_call":
      return "Tool action";
    default:
      return kind ? kind.replace(/_/g, " ") : "Event";
  }
}

export function toolLabel(toolOrCapability: string | null | undefined): string {
  if (!toolOrCapability) return "Tool action";
  const bare = toolOrCapability.replace(/^(github|gitlab)_/, "");
  const capability = bare.replace(/_/g, ".");
  return capabilityLabel(capability);
}

function cap(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

/**
 * Describe a 5-field cron expression in plain English. Falls back to the raw
 * expression when it cannot be recognized.
 */
export function cronDescription(cron: string | null | undefined): string {
  if (!cron) return "—";
  const fields = cron.trim().split(/\s+/);
  if (fields.length !== 5) return cron;

  const [minute, hour, dayOfMonth, month, dayOfWeek] = fields;
  if (dayOfMonth === "*" && month === "*" && dayOfWeek === "*") {
    if (minute.startsWith("*/") && hour === "*") {
      const minutes = minute.slice(2);
      return `Every ${minutes} minute${minutes === "1" ? "" : "s"}`;
    }
    if (minute === "0" && hour.startsWith("*/")) {
      const hours = hour.slice(2);
      return `Every ${hours} hour${hours === "1" ? "" : "s"}`;
    }
    if (minute === "0" && hour === "*") return "Every hour";
    if (minute === "0" && hour === "12") return "Every day at noon";
    if (minute === "0" && hour === "0") return "Every day at midnight";
    if (minute === "0" && hour !== "*") return `Every day at ${formatHour(hour)}`;
  }
  if (dayOfWeek !== "*" && minute === "0" && hour !== "*") {
    return `Every ${dayOfWeek} at ${formatHour(hour)}`;
  }
  return cron;
}

function formatHour(value: string): string {
  const number = Number(value);
  if (Number.isNaN(number)) return value;
  const suffix = number < 12 ? "am" : "pm";
  const hour12 = number % 12 === 0 ? 12 : number % 12;
  return `${hour12}${suffix}`;
}
