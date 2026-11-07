import {defineConfig, mergeConfig} from "vitest/config";
import viteConfig from "./vite.config";

/** 测试配置 */
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "node",
      include: ["tests/**/*.test.ts"],
    },
  }),
);
