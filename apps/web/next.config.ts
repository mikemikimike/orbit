import path from "node:path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Required for standalone output in a pnpm monorepo.
  outputFileTracingRoot: path.join(__dirname, "../../"),
  // Container-friendly output: a self-contained server bundle.
  output: "standalone",
  reactStrictMode: true,
  transpilePackages: ["@orbit/api-client", "@orbit/contracts"],
  poweredByHeader: false,
  typescript: {
    ignoreBuildErrors: false,
  },
  async headers() {
    return [
      {
        source: "/sw.js",
        headers: [
          { key: "Cache-Control", value: "no-cache, no-store, must-revalidate" },
          { key: "Service-Worker-Allowed", value: "/" },
        ],
      },
      {
        source: "/:path((?!_next/static).*)",
        headers: [
          { key: "Cache-Control", value: "no-cache, no-store, must-revalidate" },
          { key: "Pragma", value: "no-cache" },
        ],
      },
    ];
  },
};

export default nextConfig;
