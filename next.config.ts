import type { NextConfig } from "next";
import { version } from "./package.json";

const nextConfig: NextConfig = {
  output: "export",
  distDir: "dist",
  trailingSlash: true,
  skipTrailingSlashRedirect: true,
  env: {
    APP_VERSION: version,
  },
  experimental: {
    optimizePackageImports: ["@radix-ui", "lucide-react"],
  },
};

export default nextConfig;
