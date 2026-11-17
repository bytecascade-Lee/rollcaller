import {strict as assert} from "node:assert";
import {test} from "vitest";

import type {Severity} from "$types/Severity";
import type {UpdateView} from "$types/UpdateView";
import {isForced, shouldAutoOpen} from "$utils/updateOpen";

/**
 * 弹窗自动打开的规则只有一条：**critical 的候选更新**会打断用户，其余一律只改标题栏图标。
 *
 * 这里钉的是「旧行为不再回来」：`checking → available`（开机自动检查）与
 * `downloading → downloaded`（后台下载完成）都不该自己弹。
 */
function candidateData(severity: Severity) {
  return {
    info: {version: "0.9.0", notes: null, date: null},
    severity,
    reason: "newer" as const,
  };
}

const available = (severity: Severity): UpdateView => ({status: "available", data: candidateData(severity)});
const downloading = (severity: Severity): UpdateView => ({status: "downloading", data: candidateData(severity)});
const downloaded = (severity: Severity): UpdateView => ({status: "downloaded", data: candidateData(severity)});
const error = (type: "check" | "download" | "install"): UpdateView => ({status: "error", data: {type, message: "x"}});

test("普通/重要更新不自己弹：自动检查是开机就跑的，用户没问过", () => {
  assert.equal(shouldAutoOpen("checking", available("normal")), false);
  assert.equal(shouldAutoOpen("checking", available("important")), false);
});

test("critical 一出现就强制弹（available / downloading / downloaded 三态）", () => {
  assert.equal(shouldAutoOpen("checking", available("critical")), true);
  assert.equal(shouldAutoOpen("available", downloading("critical")), true);
  assert.equal(shouldAutoOpen("downloading", downloaded("critical")), true);
});

test("启动恢复现场：首次观察就是 downloaded 的 critical，仍然弹", () => {
  assert.equal(shouldAutoOpen(null, downloaded("critical")), true);
  assert.equal(shouldAutoOpen(null, downloaded("normal")), false);
});

test("下载完成本身不抢焦点", () => {
  assert.equal(shouldAutoOpen("downloading", downloaded("normal")), false);
  assert.equal(shouldAutoOpen("downloading", downloaded("important")), false);
});

test("用户已在流程里的失败必须可见，检查失败不打扰", () => {
  assert.equal(shouldAutoOpen("downloading", error("download")), true);
  assert.equal(shouldAutoOpen("downloaded", error("install")), true);
  assert.equal(shouldAutoOpen("checking", error("check")), false);
  assert.equal(shouldAutoOpen("idle", error("check")), false);
});

test("同一阶段的重复广播不再打开（用户关掉后不会被重新弹回来）", () => {
  assert.equal(shouldAutoOpen("available", available("critical")), false);
  assert.equal(shouldAutoOpen("downloading", downloading("critical")), false);
  assert.equal(shouldAutoOpen("error", error("download")), false);
});

test("中性阶段不弹", () => {
  assert.equal(shouldAutoOpen("idle", {status: "checking"}), false);
  assert.equal(shouldAutoOpen("checking", {status: "upToDate"}), false);
  assert.equal(shouldAutoOpen("upToDate", {status: "upToDate"}), false);
});

/**
 * 强制更新：界面不给关闭 / 取消 / 后台。
 *
 * 判据必须是 `severity`，因为 `reason = "force"` 只表示「critical 且 severity 是放行的必要条件」：
 * `level = Patch` + critical 会得到 `reason = "newer"`，却仍须强制。
 */
test("强制更新只看 severity：critical 的三态都算，reason 是 newer 也算", () => {
  assert.equal(isForced(available("critical")), true);
  assert.equal(isForced(downloading("critical")), true);
  assert.equal(isForced(downloaded("critical")), true);
  assert.equal(
    isForced({
      status: "available",
      data: {...candidateData("critical"), reason: "newer"},
    }),
    true,
  );
});

test("normal / important 不强制，哪怕 reason = force", () => {
  assert.equal(isForced(available("normal")), false);
  assert.equal(isForced(downloading("important")), false);
  assert.equal(
    isForced({
      status: "available",
      data: {...candidateData("important"), reason: "force"},
    }),
    false,
  );
});

test("error 永远不强制：强制不等于把用户困在死循环里", () => {
  assert.equal(isForced(error("check")), false);
  assert.equal(isForced(error("download")), false);
  assert.equal(isForced(error("install")), false);
  assert.equal(isForced({status: "idle"}), false);
  assert.equal(isForced({status: "upToDate"}), false);
});
