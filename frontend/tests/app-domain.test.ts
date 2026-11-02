import {strict as assert} from "node:assert";
import {test, vi} from "vitest";

import {appDomain} from "$routing/app/domain.svelte";

/**
 * `appDomain` 是把地址链落到浮层的唯一写入点：状态只有 `#shown`（`$state`），
 * 变化全交给 `applyTransition`。这里测的是「域」这一层，不加载任何组件。
 *
 * ⚠️ 它是模块级单例，状态在用例之间共享：每条用例都用 `freshSink()` 先落回空链。
 */
function freshSink() {
  const calls: string[] = [];
  const dispose = appDomain.install({
    open: (id, payload) => calls.push(`open:${id}:${payload === undefined ? "-" : JSON.stringify(payload)}`),
    close: (id) => calls.push(`close:${id}`),
  });
  appDomain.land([]);
  calls.length = 0;
  return {calls, dispose};
}

test("land：顺序开，载荷只交给最深的那一段", () => {
  const {calls, dispose} = freshSink();
  try {
    appDomain.land(["student-management", "edit-student"], 7);
    assert.deepEqual(appDomain.chain, ["student-management", "edit-student"]);
    assert.deepEqual(calls, ["open:student-management:-", "open:edit-student:7"]);
  } finally {
    dispose();
  }
});

test("land：退回卡片时逆序关掉浮层，链跟着变短", () => {
  const {calls, dispose} = freshSink();
  try {
    appDomain.land(["student-management", "edit-student"]);
    calls.length = 0;

    appDomain.land(["rollcall"]);
    assert.deepEqual(appDomain.chain, ["rollcall"]);
    assert.deepEqual(calls, ["close:edit-student", "close:student-management", "open:rollcall:-"]);
  } finally {
    dispose();
  }
});

test("land：同一地址重复投递靠 replay 补一次 open，浮层关了也能再点开", () => {
  const {calls, dispose} = freshSink();
  try {
    appDomain.land(["student-management", "edit-student"]);
    calls.length = 0;

    appDomain.land(["student-management", "edit-student"], {id: 3});
    assert.deepEqual(appDomain.chain, ["student-management", "edit-student"]);
    assert.deepEqual(calls, ['open:edit-student:{"id":3}']);
  } finally {
    dispose();
  }
});

test("land：没有落点表时只 warn，不抛也不改链", () => {
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
  const {dispose} = freshSink();
  try {
    appDomain.land(["rollcall"]);
    dispose();

    assert.doesNotThrow(() => appDomain.land(["student-management"]));
    assert.deepEqual(appDomain.chain, ["rollcall"], "没有落点表就不该改链");
    assert.equal(warn.mock.calls.length, 1, "应当留一条 warn");
  } finally {
    warn.mockRestore();
  }
});
