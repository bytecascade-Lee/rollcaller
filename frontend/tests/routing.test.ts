import {strict as assert} from "node:assert";
import {readFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {test} from "vitest";

import {canonicalRoute, parseAction, parseRoute, routeOf} from "$routing/address";
import {chainOf, resolveAppChain} from "$routing/app/entries";
import {createEntryRegistry} from "$routing/registry";
import {nav} from "$routing/nav";
import {applyTransition, planTransition} from "$routing/app/transition";
import {appNavigate} from "$routing/app/navigate";
import {pushAt, resetTo, stepIndex} from "$routing/help/history";
import {readSearch} from "$routing/help/payload";
import {APP_ENTRY_PARENT, AppEntry} from "$routing/app/entries.generated";
import {HelpRoute} from "$routing/help/entries.generated";

/* ---------- 打桩：node 环境没有 DOM ---------- */

/** 这些用例只用到 location/history/window 上极少几个成员，给最小桩即可 */
const env = globalThis as unknown as {
  location: { hash: string };
  history: { replaceState(): void };
  window: { addEventListener(): void; removeEventListener(): void };
};

Object.assign(env, {
  location: {hash: "#/app/rollcall"},
  history: {
    replaceState() {
    }
  },
  window: {
    addEventListener() {
    }, removeEventListener() {
    }
  },
});

const navLog = [];
/** 假域：段原样当目的地，落地时把"地址 + 载荷"记下来（等价于旧 `apply` 的观察点） */
const testDomain = {
  host: "app",
  decode: (segments) => segments,
  encode: (dest) => dest,
  land: (segments, payload) => navLog.push({route: routeOf("app", segments), payload}),
};
const installTestNav = () =>
  nav.install({domain: testDomain, forward: (action) => navLog.push({forwarded: action})});
let disposeHarness = installTestNav();

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

/* ---------- entries：链 / 地址解析 ---------- */

test("chainOf：自根到自身", () => {
  assert.deepEqual(chainOf("rollcall"), ["rollcall"]);
  assert.deepEqual(chainOf("delete-student"), ["student-management", "delete-student"]);
});

/* ---------- 整条链：校验与交付一次完成 ---------- */

test("resolveAppChain：合法地址给出整条链，非法一律 null", () => {
  assert.deepEqual(resolveAppChain(["student-management", "delete-student"]), [
    "student-management",
    "delete-student",
  ]);
  assert.deepEqual(resolveAppChain(["rollcall"]), ["rollcall"]);
  for (const segments of [
    ["rollcall", "delete-student"], // 祖先段与 parentId 不符
    ["delete-student"], // 段数少于链长
    ["student-management", "delete-student", "x"], // 多一段
    ["nope"], // 未知名
    ["toString"], // 原型属性名（名字表用 Set 的意义）
    [],
  ]) {
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

test("地址 ↔ 链 互逆：encode 出来的地址必须能被 decode 还原", () => {
  // encode = chainOf + routeOf，decode = parseRoute + resolveAppChain
  for (const entry of Object.values(AppEntry)) {
    const chain = chainOf(entry);
    const route = routeOf("app", chain);
    const parsed = parseRoute(route);
    assert.ok(parsed, `${entry} 的地址应当可切分：${route}`);
    assert.deepEqual(resolveAppChain(parsed.segments), chain, `${entry} 往返后链不一致`);
  }
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
  // 退回卡片：关浮层，replay 落在平面上 —— 卡片没有 handler，open 是预期的空操作
  assert.deepEqual(planTransition(edit, card), {close: ["edit-student"], open: [], replay: "student-management"});
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
  reg.register("a", {
    open: () => {
    }
  });
  assert.doesNotThrow(() => reg.close("a"));
});

/* ---------- help 的地址栈：纯函数，不碰 rune ---------- */

test("history：追加裁掉前进段，索引跟随新尾", () => {
  let history = {entries: [], index: 0};
  history = pushAt(history, "a");
  history = pushAt(history, "b");
  history = pushAt(history, "c");
  assert.deepEqual(history, {entries: ["a", "b", "c"], index: 2});

  // 回到 b 再点击 d：c 之后的整段被裁掉
  history = {...history, index: 1};
  assert.deepEqual(pushAt(history, "d"), {entries: ["a", "b", "d"], index: 2});
});

test("history：resetTo 只留一条（送达语义）", () => {
  assert.deepEqual(resetTo("x"), {entries: ["x"], index: 0});
});

test("history：stepIndex 在边界返回 null", () => {
  const history = {entries: ["a", "b"], index: 1};
  assert.equal(stepIndex(history, -1), 0);
  assert.equal(stepIndex(history, 1), null); // 已在尾
  assert.equal(stepIndex({entries: ["a", "b"], index: 0}, -1), null); // 已在头
  assert.equal(stepIndex({entries: [], index: 0}, 1), null); // 空栈
});

/* ---------- help 载荷：宽进窄出，认不出就当没要求 ---------- */

test("readSearch：只认 {search: string}，其余退化为纯导航", () => {
  assert.equal(readSearch({search: "点名"}), "点名");
  for (const payload of [undefined, null, "点名", 3, {}, {search: ""}, {search: "   "}, {search: 3}]) {
    assert.equal(readSearch(payload), null, `应当忽略 ${JSON.stringify(payload)}`);
  }
});

/* ---------- appNavigate：地址翻译与"关闭回退"护栏 ---------- */

test("appNavigate.path：完整路径，由 parentId 链推出", () => {
  assert.equal(appNavigate.path(AppEntry.deleteStudent), "#/app/student-management/delete-student");
  assert.equal(appNavigate.path(AppEntry.rollcall), "#/app/rollcall");
});

test("appNavigate.leave：只有自己确实是当前地址目标时才回退", () => {
  const leaveAt = (hash) => {
    navLog.length = 0;
    env.location.hash = hash;
    appNavigate.leave(AppEntry.deleteStudent);
    return navLog.map((action) => action.route);
  };

  assert.deepEqual(leaveAt("#/app/student-management/delete-student"), [
    "#/app/student-management",
  ]); // 是链尾 ⇒ 回退父级
  assert.deepEqual(leaveAt("#/app/rollcall"), []); // 别处已导航 ⇒ 不动（护栏）
  assert.deepEqual(leaveAt("#/app/student-management"), []); // 已回退过 ⇒ 幂等

  navLog.length = 0;
  env.location.hash = "#/app/rollcall";
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

test("nav.go：域解释不了的地址不落地，也不写 hash", () => {
  let wrote = 0;
  const originalReplace = env.history.replaceState;
  env.history.replaceState = () => {
    wrote += 1;
  };
  const originalWarn = console.warn;
  const warnings = [];
  console.warn = (message) => warnings.push(message);
  navLog.length = 0;
  disposeHarness();

  const stop = nav.install({
    domain: {
      host: "app",
      decode: () => null,
      encode: (dest) => dest,
      land: () => navLog.push("land"),
    },
  });
  try {
    nav.go({route: "#/app/nope"});
  } finally {
    stop();
    env.history.replaceState = originalReplace;
    console.warn = originalWarn;
    disposeHarness = installTestNav();
  }

  assert.deepEqual(navLog, [], "解释不了就不该落地");
  assert.equal(wrote, 0, "解释失败的地址不该进历史");
  assert.equal(warnings.length, 1, "应当留一条 warn");
});

test("nav.start：initial 非法时继续往 hash / fallback 兜底", () => {
  navLog.length = 0;
  env.location.hash = "";
  // 非法 initial + 空 hash ⇒ 落到 fallback（旧实现会把整条首帧路径吞掉，窗口停在空链）
  const stop = nav.start({route: "#/app/student-management"}, {route: "看起来不像地址"});
  assert.deepEqual(navLog.map((action) => action.route), ["#/app/student-management"]);
  stop();

  // 合法 initial 优先于 hash
  navLog.length = 0;
  env.location.hash = "#/app/rollcall";
  const stop2 = nav.start(
    {route: "#/app/rollcall"},
    {route: "#/app/student-management/edit-student", payload: 7},
  );
  assert.deepEqual(navLog.map((action) => action.route), ["#/app/student-management/edit-student"]);
  assert.equal(navLog[0].payload, 7);
  stop2();
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
  const addresses = Object.values(HelpRoute);
  for (const address of addresses) {
    assert.match(address, /^#\/help\/[^/]+(\/[^/]+)*$/, `非法帮助地址 ${address}`);
  }
  assert.equal(new Set(addresses).size, addresses.length, "存在两个句柄指向同一地址");
});
