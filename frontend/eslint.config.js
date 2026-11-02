import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import ts from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import globals from "globals";

/**
 * ESLint 只管**语义**，格式一律交给 Prettier（见 `prettier.config.js`）。
 *
 * 所以这里刻意不写任何缩进 / 引号 / 分号 / 空格类规则：ESLint 8.53 起就把核心格式化规则
 * 标成 deprecated，它们的活现在由 Prettier 干，写在这里只会制造两套互相打架的期望。
 */

/** TS 与 Svelte 共用：解析器认得出类型，`no-undef` 只会误报，未使用变量交给 TS 版规则 */
const tsRules = {
  ...ts.configs.recommended.rules,
  "no-undef": "off",
  "no-unused-vars": "off",
  "@typescript-eslint/no-unused-vars": [
    "error",
    {argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrors: "none"},
  ],
  // 本仓库的 Set/Map 都是「造个新的再整体赋值」，不依赖原地变更，用不着 SvelteSet/SvelteMap
  "svelte/prefer-svelte-reactivity": "off",
};

export default [
  {
    // 生成物与构建产物：types 由 ts-rs 导出，routing 下两份由 codegen 生成，改不得也管不着
    ignores: ["dist/", "node_modules/", "stats.html", "src/lib/types/", "src/lib/routing/**/*.generated.ts"],
  },
  js.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    files: ["**/*.{js,mjs,cjs}"],
    languageOptions: {ecmaVersion: "latest", sourceType: "module", globals: {...globals.node}},
  },
  {
    files: ["**/*.ts"],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {...globals.browser, ...globals.node},
    },
    plugins: {"@typescript-eslint": ts},
    rules: tsRules,
  },
  {
    files: ["**/*.svelte"],
    languageOptions: {
      // `<script lang="ts">` 里的 TS 必须显式指定子解析器，否则 espree 会在类型标注处报 parse error
      parserOptions: {parser: tsParser},
    },
    plugins: {"@typescript-eslint": ts},
    rules: {
      ...tsRules,
      // 插件集成的 a11y 规则集没启用，这条会把所有 a11y 的 svelte-ignore 都判成 unused
      "svelte/no-unused-svelte-ignore": "off",
    },
  },
  {
    // 环境声明（如 `declare module "open-props/style"`）天然只能写 any
    files: ["**/*.d.ts"],
    rules: {"@typescript-eslint/no-explicit-any": "off"},
  },
];
