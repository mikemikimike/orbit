import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "ORBIT",
    short_name: "ORBIT",
    description: "Self-hosted control plane for autonomous AI agents.",
    start_url: "/",
    display: "standalone",
    background_color: "#0e0e18",
    theme_color: "#0e0e18",
    orientation: "portrait-primary",
    icons: [
      { src: "/icon.svg", sizes: "any", type: "image/svg+xml", purpose: "any" },
      { src: "/icon.svg", sizes: "any", type: "image/svg+xml", purpose: "maskable" },
    ],
  };
}
