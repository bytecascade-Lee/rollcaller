// @ts-nocheck
import {defineConfig} from "vite";
import {sveltekit} from "@sveltejs/kit/vite";
import path from "node:path";

// @ts-expect-error process is a Node.js global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  resolve: {
    alias: {
      "$asserts": path.resolve(__dirname, "./src/asserts"),
      "$components": path.resolve(__dirname, "./src/components"),
      "$lib": path.resolve(__dirname, "./src/lib"),
      "$pages": path.resolve(__dirname, "./src/pages"),
      "$styles": path.resolve(__dirname, "./src/styles"),
      "$static": path.resolve(__dirname, "./src/static"),
    }
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 14650,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
        protocol: "ws",
        host,
        port: 14651,
      }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `backend`
      ignored: ["**/backend/**"],
    },
  },
}));
