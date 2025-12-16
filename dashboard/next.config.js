/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: "standalone",
  async rewrites() {
    const gatewayUrl =
      process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8080";
    return [
      {
        source: "/api/swift/:path*",
        destination: `${gatewayUrl}/api/v1/:path*`,
      },
    ];
  },
};

module.exports = nextConfig;

