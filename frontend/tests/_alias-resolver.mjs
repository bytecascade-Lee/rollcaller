/**
 * Node 侧的最小解析钩子：把 TS/Vite 侧的 `$<name>/...` 别名映射到 `src/lib/<name>/...`，
 * 并补上 bundler 才有的"省略扩展名"解析。
 *
 * 为什么需要它：`pnpm test:help` 用裸 node 直接跑 `.ts` 源码（`--experimental-strip-types`），
 * 而
 *   - `$utils/...` 这种前缀是 Vite 的别名，Node 只认 `#` 开头的 subpath imports；
 *   - 源码里写的是 `from "$routing/address"`（无扩展名），Node 的 ESM 解析要求显式 `.ts`。
 *
 * 校验器的价值在于"能 import 生产代码"——否则它校验的是自己的复刻版、不是真相——
 * 所以这里补一个**只读**的解析映射，同时**不改应用代码的写法**，也不去改 tsconfig /
 * vite 的别名表：两边各管各的解析器。
 *
 * 表要与 `tsconfig.json` 的 `paths` 和 `vite.config.ts` 的 `resolve.alias` 保持一致
 * （新增别名时三处都要加；表故意写全，避免"某个文件恰好用了没登记的别名"这种偶发失败）。
 */
import { statSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOTS = {
  actions: "../src/lib/actions",
  assets: "../src/assets",
  commands: "../src/lib/commands",
  components: "../src/components",
  constants: "../src/lib/constants",
  controllers: "../src/lib/controllers",
  pages: "../src/pages",
  resources: "../src/lib/resources",
  routing: "../src/lib/routing",
  services: "../src/lib/services",
  static: "../src/static",
  stores: "../src/lib/stores",
  styles: "../src/styles",
  types: "../src/lib/types",
  utils: "../src/lib/utils",
};

/** `$name` 或 `$name/rest`；带其它 `$` 的写法一律不管，交回默认解析 */
const ALIAS_RE = /^\$([a-z]+)(?:\/(.*))?$/;

const isFile = (path) => {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
};

/** 按 bundler 的顺序补扩展名；都不存在时返回 null（交回默认解析，由它报错） */
function probeExtensions(url) {
  const path = fileURLToPath(url);
  for (const candidate of [path, `${path}.ts`, `${path}.js`, `${path}/index.ts`, `${path}/index.js`]) {
    if (isFile(candidate)) return pathToFileURL(candidate).href;
  }
  return null;
}

export async function resolve(specifier, context, nextResolve) {
  const matched = ALIAS_RE.exec(specifier);
  if (matched) {
    const root = ROOTS[matched[1]];
    if (root) {
      const suffix = matched[2] ? `/${matched[2]}` : "";
      const url = new URL(`${root}${suffix}`, import.meta.url);
      const probed = probeExtensions(url);
      return nextResolve(probed ?? url.href, context);
    }
  }
  return nextResolve(specifier, context);
}
