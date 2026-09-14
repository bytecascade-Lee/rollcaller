/**
 * 帮助文档地址与锚点的校验测试。
 *
 * 运行：node --experimental-strip-types tests/help-address.test.mjs
 * （Node ≥ 22.6 原生剥离 TS 类型；markdown-it 为前端直接依赖）
 *
 * 两层内容：
 *   1) HelpAddress 单元测试：三个入口（of / parse / fromDocLink）与路由序列化；
 *   2) 文档静态校验：注解无残留、`{#id}` 合法且唯一、所有 fragment 都能落到真实锚点。
 *      这类不变量放在这里而不是渲染期，渲染期只负责"降级得体"（占位提示 / 回文档顶部）。
 */
import {strict as assert} from "node:assert";
import {readdirSync, readFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import MarkdownIt from "markdown-it";
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

const linksOf = (source) =>
  [...source.matchAll(LINK_RE)].map((m) => ({text: m[1], target: m[2]}))
    .filter((l) => l.target.includes("#") && !/^https?:\/\//i.test(l.target));

let passed = 0;
const check = (name, fn) => {
  fn();
  passed += 1;
  console.log(`  ok  ${name}`);
};

console.log("— HelpAddress：构造与序列化 —");
check("of 产出路由，锚点可选", () => {
  assert.equal(HelpAddress.of("batch-import").toHash(), "#/HELP/batch-import");
  assert.equal(HelpAddress.of("batch-import", "import-steps").toHash(), "#/HELP/batch-import/import-steps");
  assert.equal(HelpAddress.of("batch-import", "").anchor, null);
  assert.throws(() => HelpAddress.of(""));
});
check("PREFIX 为大写 #/HELP", () => {
  assert.equal(HelpAddress.PREFIX, "#/HELP");
});

console.log("— HelpAddress：解析路由 —");
check("解析带/不带 # 均可", () => {
  for (const raw of ["#/HELP/add-student/operation-steps", "/HELP/add-student/operation-steps"]) {
    const a = HelpAddress.parse(raw);
    assert.equal(a.doc, "add-student");
    assert.equal(a.anchor, "operation-steps");
  }
});
check("省略 doc 段落到默认文档", () => {
  assert.equal(HelpAddress.parse("#/HELP").doc, HelpAddress.DEFAULT_DOC);
  assert.equal(HelpAddress.parse("#/HELP/").doc, HelpAddress.DEFAULT_DOC);
});
check("前缀命中即返回地址（绝不 null，避免放行导致 404）", () => {
  for (const raw of ["#/HELP", "#/HELP/x", "#/HELP/中文", "#/HELP/a/b/c"]) {
    assert.notEqual(HelpAddress.parse(raw), null, raw);
  }
});
check("非帮助路由返回 null", () => {
  for (const raw of ["#v0-8-1", "#/APP/students", "https://example.com", "", "#/HELPX"]) {
    assert.equal(HelpAddress.parse(raw), null, raw);
  }
});
check("非 kebab 锚点降级为文档顶部", () => {
  assert.equal(HelpAddress.parse("#/HELP/add-student/操作步骤").anchor, null);
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
    const hrefs = [...html.matchAll(/href="(#\/HELP[^"]*)"/g)].map((m) => m[1]);
    assert.deepEqual(hrefs, [
      "#/HELP/add-student/operation-steps",
      "#/HELP/batch-import/import-steps",
      "#/HELP/single-rollcall/operation-steps",
      "#/HELP/auto-finish/operation-steps",
      "#/HELP/edit-record/operation-steps",
      "#/HELP/export-record/operation-steps",
      "#/HELP/edit-student/operation-steps",
      "#/HELP/delete-student/operation-steps",
    ]);
  });
  check("外部链接保持原 href（由点击期交给系统浏览器）", () => {
    const ext = /<a href="([^"]+)">Microsoft WebView2 下载页面<\/a>/.exec(html);
    assert.ok(ext, "未找到外部链接");
    assert.ok(ext[1].startsWith("https://developer.microsoft.com"));
    assert.ok(!ext[1].startsWith("#/HELP"));
  });
}
{
  const html = render(sourceOf("RELEASE_NOTES.md"));
  check("发布说明目录为同文档锚点（不改写，交浏览器原生）", () => {
    const toc = [...html.matchAll(/<a href="(#v0-[^"]*)">([^<]+)<\/a>/g)];
    assert.ok(toc.length >= 15, `目录项过少：${toc.length}`);
    for (const [, href] of toc) assert.ok(!href.startsWith("#/HELP"), `${href} 被误改写`);
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

console.log(`\n全部通过：${passed} 项断言（覆盖 ${allDocIds.length} 个文档）`);
