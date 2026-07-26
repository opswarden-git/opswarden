import type { MetadataRoute } from "next";
import en from "@/messages/en.json";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "OpsWarden",
    short_name: "OpsWarden",
    description: en.Metadata.description,
    start_url: "/en",
    scope: "/",
    display: "standalone",
    background_color: "#15161A",
    theme_color: "#15161A",
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
