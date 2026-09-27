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

import {canonicalRoute, parseRoute, routeOf} from "$routing/address.ts";
import {chainOf, readStudentIds, resolveAppEntry} from "$routing/entries.ts";
import {createEntryRegistry} from "$routing/registry.ts";
import {WindowRouter} from "$routing/windowRouter.ts";
import {appNavigate} from "$routing/appNavigate.ts";
import {APP_ENTRY_PARENT, AppEntry, HelpEntry} from "$routing/entries.generated.ts";

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
WindowRouter.install({go: (action) => navLog.push(action), start: () => () => {}});

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

test("readStudentIds：非法给空数组，混入的无效项被丢掉", () => {
  assert.deepEqual(readStudentIds({ids: [1, 2]}), [1, 2]);
  assert.deepEqual(readStudentIds({ids: ["3", 1.5, 7]}), [7]);
  for (const payload of [undefined, null, {}, {ids: "x"}, {ids: 3}]) {
    assert.deepEqual(readStudentIds(payload), [], `应当拒绝 ${JSON.stringify(payload)}`);
  }
});

/* ---------- registry：解释器先于 handler，且容错不抛 ---------- */

test("registry：interpret 先于 open；unregister 后归零", () => {
  const calls = [];
  const reg = createEntryRegistry();
  reg.register("a", {open: () => calls.push("open")});
  reg.interpret("a", () => calls.push("interpret"));
  reg.open("a");
  assert.deepEqual(calls, ["interpret", "open"]);

  reg.unregister("a");
  assert.equal(reg.has("a"), false);
  assert.equal(reg.isVisible("a"), false);
});

test("registry：未知 id 与缺省的 close/isVisible 都不抛", () => {
  const reg = createEntryRegistry();
  assert.doesNotThrow(() => reg.open("nope"));
  assert.doesNotThrow(() => reg.close("nope"));
  reg.register("a", {open: () => {}});
  assert.equal(reg.isVisible("a"), false);
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

test("appNavigate.enter：载荷随 Action 一起投递", () => {
  navLog.length = 0;
  appNavigate.enter(AppEntry.deleteStudent, {ids: [1, 2]});
  assert.deepEqual(navLog, [
    {route: "#/app/student-management/delete-student", payload: {ids: [1, 2]}},
  ]);
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
