/**
 * 帮助文档锚点注解（@section / @link）渲染管线测试。
 *
 * 运行：node --experimental-strip-types tests/help-annotations.test.mjs
 * （Node ≥ 22.6 原生剥离 TS 类型；markdown-it 为前端直接依赖）
 *
 * 覆盖：注释剥离、@section 注入、@link 顺序对应、列表连续性、
 * 未标注文档零影响、畸形注解告警、搜索文本剥离。
 */
import {strict as assert} from "node:assert";
import {readFileSync, readdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import MarkdownIt from "markdown-it";
import {stripAnnotations, attachAnnotations} from "../src/lib/utils/helpAnnotations.ts";
import {classifyLink} from "../src/lib/utils/helpLinks.ts";

const HELP_DIR = fileURLToPath(new URL("../../resources/help/", import.meta.url));

/** 复刻 MarkdownView 渲染管线：仅注解注入（不含 DOMPurify 与链接分类） */
function render(md) {
  const ann = stripAnnotations(md);
  const parser = new MarkdownIt({html: true, linkify: true, typographer: true});
  parser.core.ruler.push("help-annotations", (state) => attachAnnotations(ann, state.tokens));
  return parser.render(ann.clean);
}

/** 复刻 MarkdownView 完整渲染器规则：链接分类 + href 重写（共享同一 classifyLink；docId 激活同文档锚点） */
function renderFull(md, docId) {
  const ann = stripAnnotations(md);
  const parser = new MarkdownIt({html: true, linkify: true, typographer: true});
  parser.core.ruler.push("help-annotations", (state) => attachAnnotations(ann, state.tokens));
  const defaultLinkOpen =
    parser.renderer.rules.link_open ??
    ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));
  parser.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const action = classifyLink(String(token.attrGet("href") ?? ""), docId);
    if (action?.kind === "docs") {
      token.attrSet("href", "#");
      token.attrSet("data-action", "docs");
      token.attrSet("data-id", action.id);
    } else if (action?.kind === "external") {
      token.attrSet("data-action", "external");
    }
    return defaultLinkOpen(tokens, idx, options, env, self);
  };
  return parser.render(ann.clean);
}

const readDoc = (id) => readFileSync(`${HELP_DIR}${id}/${id}-zh-CN.md`, "utf8");

let passed = 0;
const check = (name, fn) => {
  fn();
  passed += 1;
  console.log(`  ok  ${name}`);
};

console.log("— quick-start（temp 版即正式版）—");
{
  const html = render(readDoc("quick-start"));
  // 注释行全部剥离，无残留
  check("无 [//]: # 残留", () => assert.ok(!html.includes("[//]: #")));
  // @section 注入标题
  check("标题注入 data-section", () => {
    assert.ok(html.includes('<h2 data-section="install-app">'));
    assert.ok(html.includes('<h2 data-section="prepare">'));
    assert.ok(html.includes('<h2 data-section="start-rollcall">'));
  });
  // 准备学生名单的两条列表项同属一个 <ul>（注释行不打断列表）
  check("列表连续性保持（两条目同一 ul）", () => {
    assert.equal((html.match(/<ul>/g) ?? []).length, 2); // 安装应用 + 准备学生名单
    const a = html.indexOf("逐个添加");
    const b = html.indexOf("批量导入");
    assert.ok(a !== -1 && b !== -1 && a < b);
    assert.ok(!html.slice(a, b).includes("</ul>"));
  });
  // @link 按顺序附着到带 # 的链接（外部链接无注解，过滤掉）
  check("@link 注入与顺序对应", () => {
    const links = [...html.matchAll(/<a([^>]*)>([^<]+)<\/a>/g)]
      .map((m) => ({
        section: /data-section="([^"]+)"/.exec(m[1])?.[1] ?? null,
        text: m[2],
      }))
      .filter((l) => l.section !== null);
    const expected = [
      {section: "operation-steps", text: "添加单个学生"},
      {section: "import-steps", text: "批量导入学生"},
      {section: "operation-steps", text: "单次点名"},
      {section: "operation-steps", text: "连续点名"},
      {section: "operation-steps", text: "修改记录"},
      {section: "operation-steps", text: "导出记录"},
      {section: "operation-steps", text: "修改学生信息"},
      {section: "operation-steps", text: "删除学生"},
    ];
    assert.deepEqual(links, expected);
  });
}

console.log("— 带锚点的目标文档 —");
for (const [id, name] of [
  ["add-student", "operation-steps"],
  ["single-rollcall", "operation-steps"],
  ["auto-finish", "operation-steps"],
  ["edit-record", "operation-steps"],
  ["export-record", "operation-steps"],
  ["edit-student", "operation-steps"],
  ["delete-student", "operation-steps"],
  ["manual-pause", "operation-steps"],
  ["export-student", "operation-steps"],
]) {
  check(`${id} 操作步骤锚点`, () => {
    const html = render(readDoc(id));
    assert.ok(html.includes(`<h2 data-section="${name}">`), `${id} 缺 data-section`);
    assert.ok(!html.includes("[//]: #"), `${id} 注释未剥离`);
  });
}
check("batch-import 导入步骤锚点", () => {
  const html = render(readDoc("batch-import"));
  assert.ok(html.includes('<h2 data-section="import-steps">'));
  assert.ok(!html.includes("[//]: #"));
});

console.log("— 未标注文档零影响 —");
{
  const md = "# 标题\n\n正文 [链接](../add-student/add-student-zh-CN.md) 与 [外部](https://example.com) 。";
  const html = render(md);
  check("无注解 markdown 不注入 data-section", () => assert.ok(!html.includes("data-section")));
}

console.log("— 完整渲染器集成（MarkdownView 规则）—");
{
  const html = renderFull(readDoc("quick-start"));
  const anchors = [...html.matchAll(/<a([^>]*)>([^<]+)<\/a>/g)]
    .filter((m) => m[1].includes('data-action="docs"'))
    .map((m) => ({
      id: /data-id="([^"]+)"/.exec(m[1])?.[1] ?? null,
      section: /data-section="([^"]+)"/.exec(m[1])?.[1] ?? null,
      href: /href="([^"]*)"/.exec(m[1])?.[1] ?? null,
      text: m[2],
    }));
  check("docs 链接统一重写为 # 并携带 data-id/data-section", () => {
    assert.deepEqual(anchors, [
      {id: "add-student", section: "operation-steps", href: "#", text: "添加单个学生"},
      {id: "batch-import", section: "import-steps", href: "#", text: "批量导入学生"},
      {id: "single-rollcall", section: "operation-steps", href: "#", text: "单次点名"},
      {id: "auto-finish", section: "operation-steps", href: "#", text: "连续点名"},
      {id: "edit-record", section: "operation-steps", href: "#", text: "修改记录"},
      {id: "export-record", section: "operation-steps", href: "#", text: "导出记录"},
      {id: "edit-student", section: "operation-steps", href: "#", text: "修改学生信息"},
      {id: "delete-student", section: "operation-steps", href: "#", text: "删除学生"},
    ]);
  });
  check("外部链接标记 action 且保留 href", () => {
    const ext = /<a([^>]*)>Microsoft WebView2 下载页面<\/a>/.exec(html)?.[1] ?? "";
    assert.ok(ext.includes('data-action="external"'));
    assert.ok(ext.includes('href="https://developer.microsoft.com'));
  });
  check("带 fragment 但无注解的链接降级为普通文档跳转", () => {
    const md = "详见 [说明](../add-student/add-student-zh-CN.md#操作步骤) 。";
    const out = renderFull(md);
    const a = /<a([^>]*)>说明<\/a>/.exec(out)?.[1] ?? "";
    assert.ok(a.includes('data-action="docs"'));
    assert.ok(a.includes('data-id="add-student"'));
    assert.ok(!a.includes("data-section"));
  });
  check("README 特殊链接归类为文档", () => {
    assert.deepEqual(classifyLink("../../../README.md"), {kind: "docs", id: "README.md"});
    assert.deepEqual(classifyLink("../../../CHANGELOG.md"), {kind: "docs", id: "CHANGELOG.md"});
  });
}

console.log("— 边界与降级 —");
check("同行多链接：无 # 链接不参与计数", () => {
  const md = [
    "[//]: # (@link: operation-steps)",
    "先看 [说明](../batch-import/batch-import-zh-CN.md) ，再看 [步骤](../add-student/add-student-zh-CN.md#x) 。",
  ].join("\n");
  const html = render(md);
  const anchored = /<a[^>]*>步骤<\/a>/.exec(html)?.[0] ?? "";
  assert.ok(anchored.includes('data-section="operation-steps"'));
  const plain = /<a[^>]*>说明<\/a>/.exec(html)?.[0] ?? "";
  assert.ok(!plain.includes("data-section"));
});
check("@section 中间夹正文则失效", () => {
  const md = ["[//]: # (@section: broken)", "正文内容", "## 标题"].join("\n");
  const html = render(md);
  assert.ok(!html.includes('data-section="broken"'));
});
check("畸形注解被剥离并告警", () => {
  const md = "[//]: # (@link: 中文名)\n\n## 标题";
  const warnings = [];
  const origWarn = console.warn;
  console.warn = (msg) => warnings.push(msg);
  try {
    const html = render(md);
    assert.ok(!html.includes("[//]: #"));
    assert.equal(warnings.length, 1);
  } finally {
    console.warn = origWarn;
  }
});
check("搜索索引剥离正则生效", () => {
  const md = readDoc("quick-start");
  const clean = md
    .replace(/^\s*\[\/\/\]:\s*#\s*\(@(?:section|link):[^)]*\)\s*$/gm, "")
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/\s+/g, " ");
  assert.ok(!clean.includes("@section"));
  assert.ok(!clean.includes("@link"));
  assert.ok(!clean.includes("operation-steps"));
  assert.ok(clean.includes("快速开始"));
});

console.log("— RELEASE_NOTES 目录跳转 —");
{
  const rn = readFileSync(new URL("../../RELEASE_NOTES.md", import.meta.url), "utf8");
  const html = renderFull(rn, "RELEASE_NOTES.md");
  check("12 个版本标题注入 data-section", () => {
    for (const v of [
      "v0-6-0", "v0-5-0", "v0-4-3", "v0-4-2", "v0-4-1", "v0-4-0",
      "v0-3-0", "v0-3-0-rc-2", "v0-3-0-rc-1", "v0-2-0", "v0-1-0", "v0-1-0-rc-1",
    ]) {
      assert.ok(html.includes(`<h2 data-section="${v}">`), `缺 ${v}`);
    }
  });
  check("目录项链接为同文档锚点跳转", () => {
    const toc = [...html.matchAll(/<a([^>]*)>([^<]+)<\/a>/g)]
      .map((m) => ({attrs: m[1], text: m[2]}))
      .filter((l) => /^0\.\d/.test(l.text));
    assert.equal(toc.length, 12);
    for (const l of toc) {
      assert.ok(l.attrs.includes('data-id="RELEASE_NOTES.md"'), `${l.text} 缺 data-id`);
      assert.ok(l.attrs.includes('data-section="v0-'), `${l.text} 缺 data-section`);
      assert.ok(l.attrs.includes('href="#"'), `${l.text} 未重写 href`);
    }
  });
  check("RELEASE_NOTES 无注释残留", () => assert.ok(!html.includes("[//]: #")));
}

console.log("— 交叉链接完整性校验（构建脚本的轻量版）—");
{
  // 扫描全部帮助文档 + RELEASE_NOTES：每个带 data-section 的链接，目标文件必须存在对应 @section
  const sectionCache = new Map();
  const sectionsOf = (id) => {
    if (sectionCache.has(id)) return sectionCache.get(id);
    let sections = new Set();
    try {
      const src =
        id === "RELEASE_NOTES.md"
          ? readFileSync(new URL("../../RELEASE_NOTES.md", import.meta.url), "utf8")
          : readDoc(id);
      for (const m of src.matchAll(/\[\/\/\]:\s*#\s*\(@section:\s*([a-z0-9-]+)\s*\)/g)) {
        sections.add(m[1]);
      }
    } catch {
      /* 特殊文档（README 等）不参与校验 */
    }
    sectionCache.set(id, sections);
    return sections;
  };

  const docIds = [
    ...readdirSync(HELP_DIR, {withFileTypes: true})
      .filter((e) => e.isDirectory())
      .map((e) => e.name),
    "RELEASE_NOTES.md",
  ];
  let broken = [];
  for (const id of docIds) {
    const html = renderFull(
      id === "RELEASE_NOTES.md"
        ? readFileSync(new URL("../../RELEASE_NOTES.md", import.meta.url), "utf8")
        : readDoc(id),
      id,
    );
    for (const m of html.matchAll(/<a([^>]*)>([^<]+)<\/a>/g)) {
      const attrs = m[1];
      if (!attrs.includes('data-action="docs"')) continue;
      const target = /data-id="([^"]+)"/.exec(attrs)?.[1];
      const section = /data-section="([^"]+)"/.exec(attrs)?.[1];
      if (!target || !section) continue;
      if (!sectionsOf(target).has(section)) {
        broken.push(`${id} → ${target}#${section}（链接文字：${m[2]}）`);
      }
    }
  }
  check("所有 @link 目标均有对应 @section", () => {
    assert.deepEqual(broken, []);
  });
  check("同文件 @section 无重复", () => {
    for (const id of docIds) {
      const src =
        id === "RELEASE_NOTES.md"
          ? readFileSync(new URL("../../RELEASE_NOTES.md", import.meta.url), "utf8")
          : readDoc(id);
      const names = [...src.matchAll(/\[\/\/\]:\s*#\s*\(@section:\s*([a-z0-9-]+)\s*\)/g)].map((m) => m[1]);
      assert.equal(new Set(names).size, names.length, `${id} 存在重复 @section`);
    }
  });
}

console.log(`\n全部通过：${passed} 项断言`);
