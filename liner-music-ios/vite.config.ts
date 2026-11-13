import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";
import pkg from "./package.json" with { type: "json" };
import { contentSecurityPolicyPlugin } from "./vite.csp.ts";

const host = process.env.TAURI_DEV_HOST;

// Tauri v2 mobile build:
//  - `base: "./"` is mandatory: WKWebView loads `index.html` from the
//    `tauri://localhost` custom scheme, so every asset URL must be relative.
//  - the Electron plugin pipeline is intentionally absent; there is no
//    main/preload bundle in a Tauri app.
export default defineConfig(({ command, mode }) => {
  if (command === "serve") {
    const devEnv = loadEnv(mode, process.cwd(), ["LINER_", "VITE_API_URL"]);
    for (const name of [
      "LINER_DEV_SIGNING_SECRET",
      "LINER_SIMULATE",
      "LINER_SIMULATE_COUNTRY",
      "VITE_API_URL",
    ]) {
      if (devEnv[name] && !process.env[name]) process.env[name] = devEnv[name];
    }
  }

  return {
    base: "./",
    define: {
      __APP_VERSION__: JSON.stringify(pkg.version),
      // The renderer checks this to distinguish a Tauri host from a plain
      // browser tab. Kept out of `import.meta.env` so it survives minification.
      __LINER_TAURI__: JSON.stringify(true),
    },
    plugins: [react(), tailwindcss(), contentSecurityPolicyPlugin()],
    clearScreen: false,
    resolve: {
      alias: {
        "@": path.resolve(import.meta.dirname, "./src"),
      },
    },
    server: {
      port: 5173,
      strictPort: true,
      host: host || false,
      hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
      watch: { ignored: ["**/src-tauri/**"] },
    },
    envPrefix: ["VITE_", "TAURI_ENV_*"],
    build: {
      outDir: "dist",
      emptyOutDir: true,
      target: "safari16",
      sourcemap: false,
      chunkSizeWarningLimit: 4096,
      rollupOptions: {
        output: {
          // Keep the initial WKWebView parse cheap: split the heavy,
          // lazily-imported visual engines away from the entry chunk.
          // Vite 8 (rolldown) only accepts a function here.
          manualChunks(id: string) {
            if (!id.includes("node_modules")) return undefined;
            if (/[\\/](react|react-dom|react-router|scheduler)[\\/]/.test(id))
              return "react";
            if (/[\\/](framer-motion|gsap|lottie-web|motion-dom|motion-utils)[\\/]/.test(id))
              return "motion";
            if (/[\\/](youtubei\.js|googlevideo|soundtouchjs)[\\/]/.test(id))
              return "media";
            return undefined;
          },
        },
      },
    },
  };
});
