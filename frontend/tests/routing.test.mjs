/**
 * 导航核心的单元测试。
 *
 * 运行：pnpm test
 *   = node --experimental-strip-types --import ./tests/_register-alias.mjs tests/routing.test.mjs
 * （裸 node 直接跑 `.ts` 源码；`$x/...` 别名由 `tests/_alias-resolver.mjs` 补。）
 *
 * 覆盖范围只限**纯逻辑**：地址原语、入口解析、落点注册表、app 域的翻译与回退、
 * 生成物与声明的一致性。`.svelte.ts`（helpNavigate）与组件里的转换语义不在范围内 ——
 * 裸 node 编译不了 rune。
 */
import {strict as assert} from "node:assert";
import {readFileSync} from "node:fs";
import {fileURLToPath} from "node:url";

import {canonicalRoute, parseAction, parseRoute, routeOf} from "$routing/address.ts";
import {chainOf, resolveAppChain, resolveAppEntry} from "$routing/entries.ts";
import {createEntryRegistry} from "$routing/registry.ts";
import {nav} from "$routing/nav.ts";
import {applyTransition, planTransition} from "$routing/transition.ts";
import {appNavigate} from "$routing/appNavigate.ts";
import {APP_ENTRY_PARENT, AppEntry} from "$routing/app.entries.generated.ts";
import {HelpEntry} from "$routing/help.entries.generated.ts";

/* ---------- 极简 runner：单个失败不中断，最后汇总并给非零退出码 ---------- */

const failures = [];
let passed = 0;

function test(name, fn) {
  try {
    fn();
    passed += 1;
  } catch (error) {
    failures.push({name, error});
  }
}

/* ---------- 打桩：Node 里没有 DOM ---------- */

globalThis.location = {hash: "#/app/rollcall"};
globalThis.history = {replaceState() {}};

const navLog = [];
nav.install({
  host: "app",
  apply: (action) => navLog.push(action),
  forward: (action) => navLog.push({forwarded: action}),
});

/* ---------- address：切分 / 组装 / 规范化 ---------- */

test("parseRoute：只认应用内地址，其余给 null", () => {
  assert.deepEqual(parseRoute("#/app/student-management"), {
    host: "app",
    segments: ["student-management"],
  });
  for (const raw of ["", "#", "#/", "#foo", "https://example.com/x"]) {
    assert.equal(parseRoute(raw), null, `应当拒绝 ${JSON.stringify(raw)}`);
  }
});

test("parseRoute：host 归一小写、空段被丢弃", () => {
  assert.deepEqual(parseRoute("#/APP/a//b/"), {host: "app", segments: ["a", "b"]});
});

test("routeOf / canonicalRoute：组装、滤空段、幂等", () => {
  assert.equal(routeOf("HELP", ["README.md"]), "#/help/README.md");
  assert.equal(routeOf("help"), "#/help");
  assert.equal(routeOf("help", ["a", "", "b"]), "#/help/a/b");
  for (const raw of ["#/app", "#/help/tts/modes", "#/HELP//a/"]) {
    const once = canonicalRoute(raw);
    assert.equal(canonicalRoute(once), once, `${raw} 二次规范化后变了`);
  }
  assert.equal(canonicalRoute("https://example.com"), null);
});

/* ---------- entries：链 / 地址解析 / 载荷读取 ---------- */

test("chainOf：自根到自身", () => {
  assert.deepEqual(chainOf("rollcall"), ["rollcall"]);
  assert.deepEqual(chainOf("delete-student"), ["student-management", "delete-student"]);
});

test("resolveAppEntry：父链严格，任何不符都降级为 null", () => {
  assert.equal(resolveAppEntry(["student-management", "delete-student"]), "delete-student");
  assert.equal(resolveAppEntry(["rollcall"]), "rollcall");
  for (const segments of [
    ["rollcall", "delete-student"], // 祖先段与 parentId 不符
    ["delete-student"], // 段数少于链长
    ["student-management", "delete-student", "x"], // 多一段
    ["nope"], // 未知名
    ["toString"], // 原型属性名（名字表用 Set 的意义）
    [],
  ]) {
    assert.equal(resolveAppEntry(segments), null, `应当拒绝 ${JSON.stringify(segments)}`);
  }
});

/* ---------- 整条链：校验与交付一次完成 ---------- */

test("resolveAppChain：合法地址给出整条链，非法一律 null", () => {
  assert.deepEqual(resolveAppChain(["student-management", "delete-student"]), [
    "student-management",
    "delete-student",
  ]);
  assert.deepEqual(resolveAppChain(["rollcall"]), ["rollcall"]);
  for (const segments of [["rollcall", "delete-student"], ["nope"], []]) {
    assert.equal(resolveAppChain(segments), null, `应当拒绝 ${JSON.stringify(segments)}`);
  }
});

test("parseAction：一次切分到位，规范化且带 host/segments", () => {
  assert.deepEqual(parseAction({route: "#/APP/a//b/", payload: 3}), {
    route: "#/app/a/b",
    host: "app",
    segments: ["a", "b"],
    payload: 3,
  });
  assert.equal(parseAction({route: "https://example.com"}), null);
});

/* ---------- 转换语义：共同前缀 diff ---------- */

test("planTransition：四种情形", () => {
  const card = ["student-management"];
  const edit = ["student-management", "edit-student"];
  const del = ["student-management", "delete-student"];

  assert.deepEqual(planTransition([], edit), {close: [], open: edit, replay: null});
  assert.deepEqual(planTransition(edit, del), {close: ["edit-student"], open: ["delete-student"], replay: null});
  assert.deepEqual(planTransition(edit, ["record-history", "edit-record"]), {
    close: ["edit-student", "student-management"],
    open: ["record-history", "edit-record"],
    replay: null,
  });
  // 重放同一地址：diff 无动作，靠 replay 补一次 open，否则"地址在目标、浮层已关"时点了没反应
  assert.deepEqual(planTransition(edit, edit), {close: [], open: [], replay: "edit-student"});
  assert.deepEqual(planTransition(card, card), {close: [], open: [], replay: "student-management"});
});

test("applyTransition：逆序关、顺序开，载荷只给最深的那个", () => {
  const calls = [];
  const registry = {
    close: (id) => calls.push(`close:${id}`),
    open: (id, payload) => calls.push(`open:${id}:${payload === undefined ? "-" : payload}`),
  };
  const next = applyTransition(registry, ["student-management", "edit-student"], ["record-history", "edit-record"], 7);
  assert.deepEqual(next, ["record-history", "edit-record"]);
  assert.deepEqual(calls, [
    "close:edit-student",
    "close:student-management",
    "open:record-history:-",
    "open:edit-record:7",
  ]);
});

/* ---------- registry 的载荷透传见上；载荷形状由 meta.json 声明、tsc 校验 ---------- */
/* ---------- registry：注册/打开/注销，未知 id 不抛 ---------- */

test("registry：open 转给 handler，注销后归零", () => {
  const calls = [];
  const reg = createEntryRegistry();
  reg.register("a", {open: (payload) => calls.push(payload)});
  reg.open("a", 7);
  assert.deepEqual(calls, [7]);

  reg.unregister("a");
  assert.equal(reg.has("a"), false);
});

test("registry：未知 id 与缺省的 close 都不抛", () => {
  const reg = createEntryRegistry();
  assert.doesNotThrow(() => reg.open("nope"));
  assert.doesNotThrow(() => reg.close("nope"));
  reg.register("a", {open: () => {}});
  assert.doesNotThrow(() => reg.close("a"));
});

/* ---------- appNavigate：地址翻译与"关闭回退"护栏 ---------- */

test("appNavigate.path：完整路径，由 parentId 链推出", () => {
  assert.equal(appNavigate.path(AppEntry.deleteStudent), "#/app/student-management/delete-student");
  assert.equal(appNavigate.path(AppEntry.rollcall), "#/app/rollcall");
});

test("appNavigate.leave：只有自己确实是当前地址目标时才回退", () => {
  const leaveAt = (hash) => {
    navLog.length = 0;
    globalThis.location.hash = hash;
    appNavigate.leave(AppEntry.deleteStudent);
    return navLog.map((action) => action.route);
  };

  assert.deepEqual(leaveAt("#/app/student-management/delete-student"), [
    "#/app/student-management",
  ]); // 是链尾 ⇒ 回退父级
  assert.deepEqual(leaveAt("#/app/rollcall"), []); // 别处已导航 ⇒ 不动（护栏）
  assert.deepEqual(leaveAt("#/app/student-management"), []); // 已回退过 ⇒ 幂等

  navLog.length = 0;
  globalThis.location.hash = "#/app/rollcall";
  appNavigate.leave(AppEntry.rollcall);
  assert.deepEqual(navLog, []); // 根入口无父级 ⇒ 不动
});

test("appNavigate.enter：载荷随 Action 一起投递，且就是目标本身", () => {
  navLog.length = 0;
  appNavigate.enter(AppEntry.deleteStudent, [1, 2]);
  assert.deepEqual(navLog, [
    {route: "#/app/student-management/delete-student", payload: [1, 2]},
  ]);

  navLog.length = 0;
  appNavigate.enter(AppEntry.editStudent, 7);
  assert.deepEqual(navLog, [
    {route: "#/app/student-management/edit-student", payload: 7},
  ]);
});

test("nav.go：不归本窗口的地址交给 forward，不落地", () => {
  navLog.length = 0;
  nav.go({route: "#/help/tts"});
  assert.equal(navLog.length, 1);
  assert.equal(navLog[0].forwarded.route, "#/help/tts");
});

/* ---------- 生成物 ↔ 声明：防漂移 ---------- */

const APP_META = JSON.parse(
  readFileSync(fileURLToPath(new URL("../src/lib/resources/app/meta.json", import.meta.url)), "utf8"),
);

test("生成物 app 段与 app/meta.json 一致", () => {
  assert.deepEqual(Object.keys(APP_ENTRY_PARENT), Object.keys(APP_META.nodes));
  for (const [id, parent] of Object.entries(APP_ENTRY_PARENT)) {
    assert.equal(parent, APP_META.nodes[id].parentId, `${id} 的 parentId 不一致`);
  }
  assert.deepEqual(
    Object.values(AppEntry).slice().sort(),
    Object.keys(APP_ENTRY_PARENT).slice().sort(),
    "句柄表与入口名表不是同一集合",
  );
});

test("生成物 help 段：地址格式合法且不重复", () => {
  const addresses = Object.values(HelpEntry);
  for (const address of addresses) {
    assert.match(address, /^#\/help\/[^/]+(\/[^/]+)*$/, `非法帮助地址 ${address}`);
  }
  assert.equal(new Set(addresses).size, addresses.length, "存在两个句柄指向同一地址");
});

/* ---------- 汇总 ---------- */

for (const {name, error} of failures) {
  console.error(`FAIL ${name}\n  ${error.message}`);
}
console.log(`\n${passed} passed, ${failures.length} failed`);
process.exitCode = failures.length > 0 ? 1 : 0;
