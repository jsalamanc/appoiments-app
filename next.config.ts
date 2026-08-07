import type { NextConfig } from "next";
import { withSerwist } from "@serwist/turbopack";

const nextConfig: NextConfig = {
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "jtblraeotyknhqcynutg.supabase.co",
        pathname: "/storage/v1/object/public/**",
      },
    ],
  },
};

export default withSerwist(nextConfig);
