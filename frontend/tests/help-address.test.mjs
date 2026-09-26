/**
 * 路由与帮助文档地址的校验测试。
 *
 * 运行：pnpm test:help
 *   = node --experimental-strip-types --import ./tests/_register-alias.mjs tests/help-address.test.mjs
 * （Node ≥ 22.6 原生剥离 TS 类型；`--import` 那一步补上 Node 侧的 `$x/...` 别名解析，
 *   细节见 `tests/_alias-resolver.mjs`——没有它连 import 都过不去。）
 *
 * 三层内容：
 *   1) 通用路由原语：切分 / 组装 / 规范化 / 前缀判别（`$routing/address`）；
 *   2) HelpAddress 单元测试：三个入口（of / parse / fromDocLink）与路由序列化；
 *   3) 静态校验：文档注解无残留、`{#id}` 合法且唯一、所有 fragment 都能落到真实锚点，
 *      外加具名入口表（`HelpEntries` / `AppEntries`）里的每个落点都真实存在，
 *      以及 app 路由表与 `resources/app/meta.json` 的一致性（JSON 声明 ↔ TS 白名单，
 *      含卡片 id 与各卡的叠加层名字、可寻址集合 ↔ `AppEntries` 的叠加层入口）。
 *      这类不变量放在这里而不是渲染期，渲染期只负责"降级得体"（占位提示 / 回文档顶部）。
 */
import {strict as assert} from "node:assert";
import {readdirSync, readFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import MarkdownIt from "markdown-it";
import {canonicalRoute, isRouteHref, parseRoute, routeOf} from "$routing/address.ts";
import {AppCardIds, AppCards, AppEntries, AppOwners, HelpEntries, StudentEditFields} from "$routing/entries.ts";
import {RecordOverlays, StudentOverlays} from "$routing/overlayNames.ts";
import {HelpAddress} from "$utils/helpAddress.ts";
import {headingIdPlugin} from "$utils/markdownHeadingId.ts";

const HELP_DIR = fileURLToPath(new URL("../../resources/help/", import.meta.url));
const ROOT_DIR = fileURLToPath(new URL("../../", import.meta.url));
const ROOT_DOCS = ["RELEASE_NOTES.md", "CHANGELOG.md", "README.md"];

const LINK_RE = /(?<!!)\[([^\]]*)\]\(([^)\s]+)\)/g;
const ID_RE = /\{#([^}]*)\}/g;
const KEBAB_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

/** DOMPurify 默认 SANITIZE_DOM=true：与 document 属性重名的 id 会被静默删除 */
const DOM_RESERVED = new Set([
  "title", "location", "body", "head", "cookie", "forms", "images", "links", "domain",
  "referrer", "length", "origin", "url", "style", "all", "scripts", "children", "top",
]);

/** 复刻 MarkdownView 的渲染管线：标题 id 插件 + 链接改写（外部与同文档锚点原样保留） */
function render(source) {
  const md = new MarkdownIt({html: true, linkify: true, typographer: true}).use(headingIdPlugin);
  const defaultLinkOpen = md.renderer.rules.link_open ??
    ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));
  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const address = HelpAddress.fromDocLink(String(token.attrGet("href") ?? ""));
    if (address) token.attrSet("href", address.toHash());
    return defaultLinkOpen(tokens, idx, options, env, self);
  };
  return md.render(source);
}

const docIds = readdirSync(HELP_DIR, {withFileTypes: true})
  .filter((e) => e.isDirectory())
  .map((e) => e.name)
  .sort();
const allDocIds = [...docIds, ...ROOT_DOCS];

const sourceOf = (id) => ROOT_DOCS.includes(id)
  ? readFileSync(`${ROOT_DIR}${id}`, "utf8")
  : readFileSync(`${HELP_DIR}${id}/${id}-zh-CN.md`, "utf8");

const trySourceOf = (id) => {
  try {
    return sourceOf(id);
  } catch {
    return null;
  }
};

/** 文档声明的锚点集合（`## 标题 {#id}`）；文档不存在时返回 null */
const anchorsOf = (id) => {
  const raw = trySourceOf(id);
  return raw === null ? null : new Set([...raw.matchAll(ID_RE)].map((m) => m[1]));
};

const meta = JSON.parse(readFileSync(new URL("../src/lib/resources/help/meta.json", import.meta.url), "utf8"));
const appMeta = JSON.parse(readFileSync(new URL("../src/lib/resources/app/meta.json", import.meta.url), "utf8"));
/** 卡片 id → 该卡的叠加层名字表（TS 侧） */
const APP_OVERLAYS = {
  "student-management": StudentOverlays,
  "record-history": RecordOverlays,
};
/** 落点 → 载荷字段表（TS 侧） */
const APP_PAYLOADS = {
  "student-management/edit": StudentEditFields,
};

const linksOf = (source) =>
  [...source.matchAll(LINK_RE)].map((m) => ({text: m[1], target: m[2]}))
    .filter((l) => l.target.includes("#") && !/^https?:\/\//i.test(l.target));

let passed = 0;
const check = (name, fn) => {
  fn();
  passed += 1;
  console.log(`  ok  ${name}`);
};

console.log("— 通用路由原语 —");
check("切分：host 大小写不敏感，空段丢弃，段本体原样保留", () => {
  assert.deepEqual(
    parseRoute("#/Help//Add-Student/Operation-Steps"),
    {host: "help", segments: ["Add-Student", "Operation-Steps"]},
  );
  assert.deepEqual(parseRoute("#/help"), {host: "help", segments: []});
});
check("切分：非应用内路由一律 null", () => {
  for (const raw of ["", "#", "#v0-8-1", "#/", "https://example.com", "/", "overview.md"]) {
    assert.equal(parseRoute(raw), null, raw);
  }
});
check("组装：host 归一为小写，无空段、无尾斜杠", () => {
  assert.equal(routeOf("HELP", ["README.md"]), "#/help/README.md");
  assert.equal(routeOf("help", ["a", "", "b"]), "#/help/a/b");
  assert.equal(routeOf("help"), "#/help");
});
check("规范化幂等：同一地址的 canonical 字符串必然相同", () => {
  for (const raw of ["#/HELP", "#/help/", "#/help//x", "/help/x", "#/Help/X"]) {
    const once = canonicalRoute(raw);
    assert.notEqual(once, null, raw);
    assert.equal(canonicalRoute(once), once, `${raw} → ${once} 二次规范化后变了`);
  }
});
check("前缀判别靠 `#/`，与大小写无关", () => {
  assert.equal(isRouteHref("#/help/x"), true);
  assert.equal(isRouteHref("#/HELP/x"), true);
  assert.equal(isRouteHref("#v0-8-1"), false);
  assert.equal(isRouteHref("https://example.com/#/help"), false);
});

console.log("— HelpAddress：构造与序列化 —");
check("of 产出路由，锚点可选", () => {
  assert.equal(HelpAddress.of("batch-import").toHash(), "#/help/batch-import");
  assert.equal(HelpAddress.of("batch-import", "import-steps").toHash(), "#/help/batch-import/import-steps");
  assert.equal(HelpAddress.of("batch-import", "").anchor, null);
  assert.throws(() => HelpAddress.of(""));
});
check("host 为小写（canonical 即小写），段本体原样保留", () => {
  assert.equal(HelpAddress.HOST, "help");
  assert.equal(HelpAddress.of("README.md").toHash(), "#/help/README.md");
});

console.log("— HelpAddress：解析路由 —");
check("解析带/不带 # 均可，host 大小写不敏感", () => {
  for (const raw of ["#/help/add-student/operation-steps", "/help/add-student/operation-steps", "#/HELP/add-student/operation-steps"]) {
    const a = HelpAddress.parse(raw);
    assert.equal(a.doc, "add-student", raw);
    assert.equal(a.anchor, "operation-steps", raw);
  }
});
check("省略 doc 段落到默认文档", () => {
  assert.equal(HelpAddress.parse("#/help").doc, HelpAddress.DEFAULT_DOC);
  assert.equal(HelpAddress.parse("#/help/").doc, HelpAddress.DEFAULT_DOC);
});
check("host 命中即返回地址（绝不 null，避免放行导致 404）", () => {
  for (const raw of ["#/help", "#/help/x", "#/help/中文", "#/help/a/b/c"]) {
    assert.notEqual(HelpAddress.parse(raw), null, raw);
  }
});
check("非帮助 host 返回 null", () => {
  for (const raw of ["#v0-8-1", "#/app/students", "https://example.com", "", "#/helpx", "#/help-x"]) {
    assert.equal(HelpAddress.parse(raw), null, raw);
  }
});
check("非 kebab 锚点降级为文档顶部", () => {
  assert.equal(HelpAddress.parse("#/help/add-student/操作步骤").anchor, null);
});

console.log("— HelpAddress：解析文档内链接 —");
check("相对文档链接（含语言后缀与 fragment）", () => {
  assert.deepEqual(
    (({doc, anchor}) => [doc, anchor])(HelpAddress.fromDocLink("../add-student/add-student-zh-CN.md#operation-steps")),
    ["add-student", "operation-steps"],
  );
  assert.equal(HelpAddress.fromDocLink("../tts/tts-en-US.md#modes").doc, "tts");
  assert.equal(HelpAddress.fromDocLink("../tts/tts-zh-CN.md").anchor, null);
});
check("根目录文档链接", () => {
  assert.equal(HelpAddress.fromDocLink("../../../RELEASE_NOTES.md#v0-8-1").doc, "RELEASE_NOTES.md");
  assert.equal(HelpAddress.fromDocLink("../../../RELEASE_NOTES.md#v0-8-1").anchor, "v0-8-1");
});
check("外部链接、同文档锚点、绝对路径一律不接管", () => {
  for (const href of [
    "https://example.com/#anchor",
    "#v0-8-1",
    "/root/path.md#x",
    "mailto:a@b.c",
    "../add-student/other-name.md#x",
    "",
  ]) {
    assert.equal(HelpAddress.fromDocLink(href), null, href);
  }
});
check("非 kebab fragment 降级为文档顶部", () => {
  const a = HelpAddress.fromDocLink("../add-student/add-student-zh-CN.md#操作步骤");
  assert.equal(a.doc, "add-student");
  assert.equal(a.anchor, null);
});

console.log("— 渲染管线集成 —");
{
  const html = render(sourceOf("quick-start"));
  check("标题锚点提为 id 且不残留声明", () => {
    assert.ok(html.includes('<h2 id="install-app">安装应用</h2>'), "缺 install-app id");
    assert.ok(html.includes('<h2 id="prepare">准备名单</h2>'));
    assert.ok(html.includes('<h2 id="start-rollcall">开始点名</h2>'));
    assert.ok(!html.includes("{#"), "仍有 {#...} 残留在正文");
  });
  check("文档链接改写为应用内路由", () => {
    const hrefs = [...html.matchAll(/href="(#\/help[^"]*)"/g)].map((m) => m[1]);
    assert.deepEqual(hrefs, [
      "#/help/add-student/operation-steps",
      "#/help/batch-import/import-steps",
      "#/help/single-rollcall/operation-steps",
      "#/help/auto-finish/operation-steps",
      "#/help/edit-record/operation-steps",
      "#/help/export-record/operation-steps",
      "#/help/edit-student/operation-steps",
      "#/help/delete-student/operation-steps",
    ]);
  });
  check("外部链接保持原 href（由点击期交给系统浏览器）", () => {
    const ext = /<a href="([^"]+)">Microsoft WebView2 下载页面<\/a>/.exec(html);
    assert.ok(ext, "未找到外部链接");
    assert.ok(ext[1].startsWith("https://developer.microsoft.com"));
    assert.ok(!ext[1].startsWith("#/help"));
  });
}
{
  const html = render(sourceOf("RELEASE_NOTES.md"));
  check("发布说明目录为同文档锚点（不改写，交浏览器原生）", () => {
    const toc = [...html.matchAll(/<a href="(#v0-[^"]*)">([^<]+)<\/a>/g)];
    assert.ok(toc.length >= 15, `目录项过少：${toc.length}`);
    for (const [, href] of toc) assert.ok(!href.startsWith("#/help"), `${href} 被误改写`);
  });
  check("版本标题均带 id 且与目录 fragment 一一对应", () => {
    const headingIds = [...html.matchAll(/<h2 id="(v0-[^"]+)"/g)].map((m) => m[1]);
    const tocFragments = [...html.matchAll(/<a href="#(v0-[^"]+)"/g)].map((m) => m[1]);
    assert.equal(headingIds.length, tocFragments.length);
    for (const f of tocFragments) assert.ok(headingIds.includes(f), `目录 ${f} 无对应标题`);
  });
}

console.log("— 文档静态校验 —");
check("无 [//]: # 注解残留", () => {
  const left = allDocIds.filter((id) => sourceOf(id).includes("[//]: #"));
  assert.deepEqual(left, []);
});
check("锚点均为 kebab 且同文档唯一", () => {
  for (const id of allDocIds) {
    const ids = [...sourceOf(id).matchAll(ID_RE)].map((m) => m[1]);
    for (const one of ids) assert.ok(KEBAB_RE.test(one), `${id} 非法锚点 ${one}`);
    assert.equal(new Set(ids).size, ids.length, `${id} 存在重复锚点`);
  }
});
check("锚点避开 DOMPurify 会删除的 DOM 保留词", () => {
  for (const id of allDocIds) {
    for (const one of anchorsOf(id)) {
      assert.ok(!DOM_RESERVED.has(one), `${id} 锚点 ${one} 会被 DOMPurify 静默删除`);
    }
  }
});
/**
 * 已知的历史遗留死锚点：CHANGELOG 的 0.1.0-rc.1 段落里 `[0.1.0](#010)` 指向不存在的
 * `#010`（在 GitHub 与旧版应用内同样点不动），与本次锚点迁移无关，按"不动 CHANGELOG
 * 既有版本段"的约定暂不修改。若哪天补上了标题锚点，下面的断言会提示删掉这条豁免。
 */
const KNOWN_DEAD_LINKS = new Set(["CHANGELOG.md → #010"]);

check("所有 fragment 都能落到真实锚点", () => {
  const broken = [];
  for (const id of allDocIds) {
    for (const {target, text} of linksOf(sourceOf(id))) {
      const fragment = target.split("#").slice(1).join("#");
      const self = target.startsWith("#");
      const address = self ? null : HelpAddress.fromDocLink(target);
      if (!self) {
        if (!address) broken.push(`${id} → ${target}（链接文字：${text}）无法解析为文档链接`);
        else if (!address.anchor) broken.push(`${id} → ${target}（链接文字：${text}）fragment 非 kebab，已降级`);
        else {
          const targets = anchorsOf(address.doc);
          if (targets === null) broken.push(`${id} → ${target}（链接文字：${text}）目标文档不存在`);
          else if (!targets.has(address.anchor)) {
            broken.push(`${id} → ${address.doc}#${address.anchor}（链接文字：${text}）目标锚点不存在`);
          }
        }
        continue;
      }
      if (!KEBAB_RE.test(fragment)) broken.push(`${id} → ${target}（链接文字：${text}）同文档 fragment 非 kebab`);
      else if (!anchorsOf(id).has(fragment)) broken.push(`${id} → #${fragment}（链接文字：${text}）同文档锚点不存在`);
    }
  }
  const keyOf = (line) => `${
    line.slice(0, line.indexOf(" → "))
  } → ${
    line.slice(line.indexOf(" → ") + 3, line.indexOf("（"))
  }`;
  const ignored = broken.filter((line) => KNOWN_DEAD_LINKS.has(keyOf(line)));
  const real = broken.filter((line) => !ignored.includes(line));
  assert.deepEqual(real, []);
  assert.equal(ignored.length, KNOWN_DEAD_LINKS.size, "已知死锚点已修复，请同步删除 KNOWN_DEAD_LINKS 中的豁免");
});
check("meta.json 的节点与顺序自洽", () => {
  for (const [key, ids] of Object.entries(meta.order)) {
    for (const id of ids) assert.ok(meta.nodes[id], `order.${key} 引用了不存在的节点 ${id}`);
  }
  for (const [id, node] of Object.entries(meta.nodes)) {
    if (node.parentId) assert.ok(meta.nodes[node.parentId], `${id} 的 parentId 不存在`);
  }
});
check("所有被链接的文档都在 meta.json 中登记", () => {
  const missing = new Set();
  for (const id of allDocIds) {
    for (const {target} of linksOf(sourceOf(id))) {
      if (target.startsWith("#")) continue;
      const address = HelpAddress.fromDocLink(target);
      if (address && !meta.nodes[address.doc]) missing.add(`${id} → ${address.doc}`);
    }
  }
  assert.deepEqual([...missing], []);
});

console.log("— 具名入口表 —");
check("HelpEntries：每个落点的文档与锚点都真实存在", () => {
  for (const [name, action] of Object.entries(HelpEntries)) {
    assert.equal(canonicalRoute(action.route), action.route, `${name} 的 route 不是 canonical 形式`);
    const address = HelpAddress.parse(action.route);
    assert.notEqual(address, null, `${name} 不是合法的帮助地址`);
    assert.ok(meta.nodes[address.doc], `${name} → 文档 ${address.doc} 未在 meta.json 登记`);
    const anchors = anchorsOf(address.doc);
    assert.notEqual(anchors, null, `${name} → 文档 ${address.doc} 没有对应的 .md 文件`);
    if (address.anchor) {
      assert.ok(anchors.has(address.anchor), `${name} → ${address.doc}#${address.anchor} 锚点不存在`);
    }
  }
});
check("HelpEntries：不指向只有标题没有内容的父节点", () => {
  const contentless = new Set(["rollcall", "students", "records", "ui", "about", "multi-rollcall"]);
  for (const [name, action] of Object.entries(HelpEntries)) {
    const address = HelpAddress.parse(action.route);
    assert.ok(!contentless.has(address.doc), `${name} 指向了无内容的父节点 ${address.doc}`);
  }
});
console.log("— app 路由表 —");
check("AppEntries：结构为 `#/app/<卡片>[/<叠加层>]` 且 canonical", () => {
  for (const [name, action] of Object.entries(AppEntries)) {
    const parsed = parseRoute(action.route);
    assert.equal(parsed?.host, "app", name);
    assert.ok(parsed.segments.length >= 1, `${name} 没有卡片段`);
    assert.equal(canonicalRoute(action.route), action.route, `${name} 的 route 不是 canonical 形式`);
  }
});
check("app/meta.json 的节点与 AppCardIds 一一对应", () => {
  assert.deepEqual([...AppCardIds].sort(), Object.keys(appMeta.nodes).sort());
});
check("每张卡片都有同名入口，且首段等于该卡片 id", () => {
  const cards = Object.entries(AppEntries)
    .filter(([, action]) => parseRoute(action.route).segments.length === 1);
  assert.deepEqual(cards.map(([name]) => name).sort(), Object.keys(AppCards).sort());
  for (const [name, action] of cards) {
    assert.equal(parseRoute(action.route).segments[0], AppCards[name], `${name} 的首段与 AppCards.${name} 不一致`);
  }
});
check("每张卡片的 overlays 与 TS 名字表一一对应", () => {
  for (const [id, node] of Object.entries(appMeta.nodes)) {
    assert.deepEqual(
      Object.keys(node.overlays ?? {}).sort(),
      [...(APP_OVERLAYS[id] ?? [])].sort(),
      `卡片 ${id} 的 overlays 与 overlayNames.ts 不一致`,
    );
  }
});
check("可寻址的叠加层与 AppEntries 的叠加层入口一一对应", () => {
  const addressable = Object.entries(appMeta.nodes).flatMap(([id, node]) =>
    Object.entries(node.overlays ?? {})
      .filter(([, overlay]) => overlay.addressable)
      .map(([name]) => `${id}/${name}`));
  const entries = Object.entries(AppEntries)
    .map(([, action]) => parseRoute(action.route).segments)
    .filter((segments) => segments.length > 1)
    .map((segments) => segments.join("/"));
  assert.deepEqual(entries.sort(), addressable.sort());
});
check("owner 与 AppOwners 一一对应，且无 owner 的节点不可寻址", () => {
  const nodes = Object.entries(appMeta.nodes).flatMap(([, node]) => [
    node,
    ...Object.values(node.overlays ?? {}),
  ]);
  assert.deepEqual(
    nodes.filter((node) => node.owner !== null).map((node) => node.owner).sort(),
    [...AppOwners].sort(),
  );
  for (const node of nodes) {
    if (node.owner === null) assert.equal(node.addressable ?? false, false, "没有实现方的节点不能可寻址");
  }
});
check("声明了 payload 的落点必须可寻址，且字段名与 TS 类型一致", () => {
  const declared = Object.entries(appMeta.nodes).flatMap(([id, node]) =>
    Object.entries(node.overlays ?? {})
      .filter(([, overlay]) => overlay.payload)
      .map(([name]) => `${id}/${name}`));
  assert.deepEqual(declared.sort(), Object.keys(APP_PAYLOADS).sort());
  for (const [path, fields] of Object.entries(APP_PAYLOADS)) {
    const [card, name] = path.split("/");
    const overlay = appMeta.nodes[card]?.overlays?.[name];
    assert.equal(overlay.addressable, true, `${path} 带载荷却不可寻址`);
    assert.deepEqual([...overlay.payload].sort(), [...fields].sort(), `${path} 的 payload 字段与 TS 类型不一致`);
  }
});
check("叠加层入口落在已声明的卡片上，且无两条入口指向同一地址", () => {
  const seen = new Set();
  for (const [name, action] of Object.entries(AppEntries)) {
    const [first, ...rest] = parseRoute(action.route).segments;
    if (rest.length > 0) assert.ok(AppCardIds.includes(first), `${name} 落在了未声明的卡片 ${first}`);
    assert.ok(!seen.has(action.route), `${name} 与另一条入口指向同一地址 ${action.route}`);
    seen.add(action.route);
  }
});

console.log(`\n全部通过：${passed} 项断言（覆盖 ${allDocIds.length} 个文档）`);
