const { withVarlockMetroConfig } = require("@varlock/expo-integration/metro-config");
const { getDefaultConfig } = require("expo/metro-config");
const { withUniwindConfig } = require("uniwind/metro");
const { wrapWithReanimatedMetroConfig } = require("react-native-reanimated/metro-config");

/** @type {import('expo/metro-config').MetroConfig} */
const config = getDefaultConfig(__dirname);
config.resolver.sourceExts.push("sql");

// expo-sqlite uses a WASM worker on web; the browser also needs these
// headers for SharedArrayBuffer. Native builds ignore both.
if (!config.resolver.assetExts.includes("wasm")) {
  config.resolver.assetExts.push("wasm");
}

const enhanceMiddleware = config.server.enhanceMiddleware;
config.server.enhanceMiddleware = (middleware, metroServer) => {
  const enhanced = enhanceMiddleware
    ? enhanceMiddleware(middleware, metroServer)
    : middleware;
  return (req, res, next) => {
    res.setHeader("Cross-Origin-Embedder-Policy", "credentialless");
    res.setHeader("Cross-Origin-Opener-Policy", "same-origin");

    // expo-sqlite creates its web worker from "/worker", but Metro serves the
    // bundled worker elsewhere. Point the request at the real bundle so the
    // browser preview can open the local database.
    if (req.url === "/worker" || req.url?.startsWith("/worker?")) {
      req.url =
        "/apps/native/node_modules/expo-sqlite/web/worker.bundle?platform=web&dev=true&hot=false";
    }

    return enhanced(req, res, next);
  };
};

const uniwindConfig = withUniwindConfig(wrapWithReanimatedMetroConfig(config), {
  cssEntryFile: "./global.css",
  dtsFile: "./uniwind-types.d.ts",
});

module.exports = withVarlockMetroConfig(uniwindConfig);
