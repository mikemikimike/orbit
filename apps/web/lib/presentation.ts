import { AUTONOMY_LEVELS } from "@orbit/contracts";

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
  external_side_effect: { label: "Can act outside ORBIT", tone: "external" },
  none: { label: "No side effects", tone: "safe" },
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

/** The level's name as defined once in `@orbit/contracts` (Observe, Recommend, ...). */
export function autonomyName(level: number | null | undefined): string {
  if (level === null || level === undefined) return "Autonomy";
  return AUTONOMY_LEVELS.find((entry) => entry.level === level)?.name ?? `Level ${level}`;
}

const BRAND_NAMES: Record<string, string> = {
  github: "GitHub",
  gitlab: "GitLab",
  x: "X",
  kubernetes: "Kubernetes",
  telegram: "Telegram",
  webhook: "Webhook",
  mock: "Mock",
  opencode: "OpenCode",
  pi: "Pi",
  claude_code: "Claude Code",
};

/** Product names keep their own capitalisation ("GitLab", not "Gitlab"). */
export function brandName(slug: string | null | undefined): string {
  if (!slug) return "";
  return (
    BRAND_NAMES[slug.toLowerCase()] ??
    slug.replace(/[_-]+/g, " ").replace(/\b\w/g, (char) => char.toUpperCase())
  );
}

const IDENTIFIER_WORDS: Record<string, string> = {
  api: "API",
  id: "ID",
  url: "URL",
  oidc: "OIDC",
  oauth2: "OAuth 2",
  oauth: "OAuth",
  hmac: "HMAC",
  sso: "SSO",
  jwks: "JWKS",
  ci: "CI",
};

/**
 * Turn a machine identifier into a label: "chatId" → "Chat ID", "api_token" →
 * "API token", "execution.cancel_requested" → "Execution cancel requested".
 */
export function humanizeIdentifier(value: string | null | undefined): string {
  if (!value) return "";
  const words = value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Za-z])(\d)/g, "$1$2")
    .split(/[\s._-]+/)
    .filter(Boolean)
    .map((word) => word.toLowerCase());
  const merged: string[] = [];
  for (let index = 0; index < words.length; index += 1) {
    const pair = `${words[index]}${words[index + 1] ?? ""}`;
    if (IDENTIFIER_WORDS[pair] && words[index + 1]) {
      merged.push(IDENTIFIER_WORDS[pair]);
      index += 1;
    } else {
      merged.push(IDENTIFIER_WORDS[words[index]] ?? words[index]);
    }
  }
  const sentence = merged.join(" ");
  return sentence.charAt(0).toUpperCase() + sentence.slice(1);
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
  if (dayOfMonth === "*" && month === "*" && dayOfWeek !== "*" && minute === "0" && hour !== "*") {
    return `${describeWeekdays(dayOfWeek)} at ${formatHour(hour)} UTC`;
  }
  return cron;
}

const WEEKDAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/** "1-5" → "Every weekday", "1" → "Every Monday", "1,3" → "Every Monday and Wednesday". */
function describeWeekdays(field: string): string {
  if (field === "1-5") return "Every weekday";
  if (field === "0,6" || field === "6,0" || field === "6,7") return "Every weekend day";
  const range = field.match(/^(\d)-(\d)$/);
  if (range && WEEKDAYS[Number(range[1])] && WEEKDAYS[Number(range[2])]) {
    return `Every ${WEEKDAYS[Number(range[1])]} to ${WEEKDAYS[Number(range[2])]}`;
  }
  const days = field.split(",").map((part) => WEEKDAYS[Number(part)]);
  if (days.every(Boolean)) {
    return `Every ${days.length > 1 ? `${days.slice(0, -1).join(", ")} and ${days.at(-1)}` : days[0]}`;
  }
  return `On days ${field}`;
}

function formatHour(value: string): string {
  const number = Number(value);
  if (Number.isNaN(number)) return value;
  const suffix = number < 12 ? "am" : "pm";
  const hour12 = number % 12 === 0 ? 12 : number % 12;
  return `${hour12}${suffix}`;
}
