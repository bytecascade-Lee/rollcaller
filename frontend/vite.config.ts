import {defineConfig} from "vite";
import {sveltekit} from "@sveltejs/kit/vite";
import path from "node:path";
import {o7Icon} from "@o7/icon/vite";

const host: string | undefined = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [await sveltekit(), o7Icon()],
  resolve: {
    alias: {
      "$asserts": path.resolve(__dirname, "./src/asserts"),
      "$components": path.resolve(__dirname, "./src/components"),
      "$stores": path.resolve(__dirname, "./src/lib/stores"),
      "constants": path.resolve(__dirname, "./src/lib/stores"),
      "$types": path.resolve(__dirname, "./src/lib/types"),
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
    hmr: host ? {
      protocol: "ws",
      host,
      port: 14651,
    } : undefined,
    watch: {
      ignored: ["**/backend/**"],
    },
  },
}));
