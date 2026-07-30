import {defineConfig} from "vite";
import {svelte} from "@sveltejs/vite-plugin-svelte"
import path from "node:path";

const host: string | undefined = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [svelte()],
  resolve: {
    alias: {
      "$asserts": path.resolve(__dirname, "./src/asserts"),
      "$components": path.resolve(__dirname, "./src/components"),
      "$constants": path.resolve(__dirname, "./src/lib/constants"),
      "$pages": path.resolve(__dirname, "./src/pages"),
      "$services": path.resolve(__dirname, "./src/lib/services"),
      "$static": path.resolve(__dirname, "./src/static"),
      "$stores": path.resolve(__dirname, "./src/lib/stores"),
      "$styles": path.resolve(__dirname, "./src/styles"),
      "$types": path.resolve(__dirname, "./src/lib/types"),
      "$utils": path.resolve(__dirname, "./src/lib/utils"),
    }
  },
  optimizeDeps: {
    include: ["phosphor-svelte"]
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
