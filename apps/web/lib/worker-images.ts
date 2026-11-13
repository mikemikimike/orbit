// Default worker images published by CI. Override the registry prefix with
// NEXT_PUBLIC_ORBIT_IMAGE_PREFIX when using a mirror (e.g. a private registry).
const PREFIX =
  process.env.NEXT_PUBLIC_ORBIT_IMAGE_PREFIX?.replace(/\/$/, "") ||
  "ghcr.io/opsifyhq/orbit";

export const DEFAULT_WORKER_IMAGES: Record<string, string> = {
  pi: `${PREFIX}/orbit-worker-pi`,
  opencode: `${PREFIX}/orbit-worker-opencode`,
  mock: `${PREFIX}/orbit-worker`,
};

export function defaultWorkerImage(harness: string): string {
  return DEFAULT_WORKER_IMAGES[harness] ?? `${PREFIX}/orbit-worker`;
}
