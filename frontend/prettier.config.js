/** 前端格式化配置 */
export default {
  printWidth: 120,
  bracketSpacing: false,
  endOfLine: "lf",
  plugins: ["prettier-plugin-svelte"],
  overrides: [
    {files: "*.svelte", options: {parser: "svelte"}},
    {files: "*.css", options: {tabWidth: 4}},
    {files: ["*.json", "*.jsonc"], options: {tabWidth: 4}},
  ],
};
