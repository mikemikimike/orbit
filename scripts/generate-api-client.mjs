// Generate the typed API client from the running control plane's OpenAPI document.
// Usage: pnpm api:generate   (API must be reachable at ORBIT_API_URL)
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";

const apiUrl = (process.env.ORBIT_API_URL ?? "http://localhost:8080").replace(/\/$/, "");
const schemaUrl = `${apiUrl}/api/v1/openapi.json`;

const check = await fetch(schemaUrl).catch(() => null);
if (!check || !check.ok) {
  console.error(`Cannot reach ${schemaUrl}. Start orbit-api first (cargo run -p orbit-api-server).`);
  process.exit(1);
}

const cwd = new URL("../packages/api-client", import.meta.url).pathname;
if (!existsSync(`${cwd}/package.json`)) {
  console.error("packages/api-client not found");
  process.exit(1);
}

const result = spawnSync(
  "pnpm",
  ["exec", "openapi-typescript", schemaUrl, "-o", "src/schema.d.ts"],
  { cwd, stdio: "inherit", env: process.env },
);
process.exit(result.status ?? 1);
