import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  webpack: (config, { isServer }) => {
    // Enable WebAssembly support
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      syncWebAssembly: true,
      layers: true,
      topLevelAwait: true,
    };

    // Exclude Miden SDK from server-side bundling
    if (isServer) {
      config.externals = config.externals || [];
      if (Array.isArray(config.externals)) {
        config.externals.push("@demox-labs/miden-sdk");
      } else {
        config.externals = [config.externals, "@demox-labs/miden-sdk"];
      }
    } else {
      config.output.webassemblyModuleFilename = "static/wasm/[modulehash].wasm";
      config.output.environment = {
        ...config.output.environment,
        asyncFunction: true,
      };
    }

    // Handle .wasm files
    config.module.rules.push({
      test: /\.wasm$/,
      type: "asset/resource",
    });

    // Fix for missing 'wbg' module - ignore wasm-bindgen imports
    config.resolve.alias = {
      ...config.resolve.alias,
      wbg: false,
    };

    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      net: false,
      tls: false,
      crypto: false,
    };

    // Ignore warnings from wasm files
    config.ignoreWarnings = [
      ...(config.ignoreWarnings || []),
      /Failed to parse source map/,
      /Can't resolve 'wbg'/,
      /The generated code contains 'async\/await'/,
      /topLevelAwait/,
    ];

    return config;
  },
};

export default nextConfig;
