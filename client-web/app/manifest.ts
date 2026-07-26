import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "OpsWarden",
    short_name: "OpsWarden",
    description: "Real-time incident response and release coordination.",
    start_url: "/en",
    scope: "/",
    display: "standalone",
    background_color: "#15161A",
    theme_color: "#F4C430",
    icons: [
      {
        src: "/assets/icon-192.png",
        sizes: "192x192",
        type: "image/png",
      },
      {
        src: "/assets/icon-512.png",
        sizes: "512x512",
        type: "image/png",
      },
      {
        src: "/assets/icon-maskable-512.png",
        sizes: "512x512",
        type: "image/png",
        purpose: "maskable",
      },
    ],
  };
}
