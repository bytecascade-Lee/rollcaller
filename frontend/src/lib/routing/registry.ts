/**
 * 落点（entry）注册表：**窗口内一张表**。
 *
 * ## 谁注册
 *
 * 只有**叠加层**注册。平面的呈现是"当前地址链的链首"的投影（见 `App.svelte` 的转换语义），
 * 不需要 handler —— 这正是"合表不合槽"：统一的是**如何被寻址**，不是**如何被存储**。
 *
 * ## 契约
 *
 * 三个方法：`open`（打开这个浮层）、`close`（收起）、`isVisible`（报告可见性）。
 * `close` 不能省：**关闭不是导航** —— 不能要求用户点"取消"也走一次导航，所以它必须待在这里。
 *
 * ## payload
 *
 * payload 是**一次性命令参数**（可 JSON 序列化、不可执行）。两种消费方式：
 *
 * - 弹窗在 `open(payload)` 里把它**快照**成自己的内部变量（`delete-student` 就是这样），
 *   之后只认那个变量 —— 弹窗不读页面状态，调用方也只负责给 payload；
 * - 需要改动页面状态的（`edit-student` 要把"编辑谁"写进页面的选中集），由**页面**用
 *   `interpret` 注册解释器，在 `open` 之前执行。
 */
export type EntryHandler = {
  open: (payload?: unknown) => void;
  close?: () => void;
  isVisible?: () => boolean;
};

export type EntryRegistry<Id extends string = string> = {
  register(id: Id, handler: EntryHandler): void;
  /** 注册载荷解释器：在 `open` 之前执行（由拥有页面状态的页面注册） */
  interpret(id: Id, translate: (payload?: unknown) => void): void;
  unregister(id: Id): void;
  open(id: Id, payload?: unknown): void;
  close(id: Id): void;
  isVisible(id: Id): boolean;
  has(id: Id): boolean;
};

export function createEntryRegistry<Id extends string = string>(): EntryRegistry<Id> {
  const handlers = new Map<Id, EntryHandler>();
  const interpreters = new Map<Id, (payload?: unknown) => void>();
  return {
    register(id, handler) {
      handlers.set(id, handler);
    },
    interpret(id, translate) {
      interpreters.set(id, translate);
    },
    unregister(id) {
      handlers.delete(id);
      interpreters.delete(id);
    },
    open(id, payload) {
      interpreters.get(id)?.(payload);
      handlers.get(id)?.open(payload);
    },
    close(id) {
      handlers.get(id)?.close?.();
    },
    isVisible(id) {
      return handlers.get(id)?.isVisible?.() ?? false;
    },
    has(id) {
      return handlers.has(id);
    },
  };
}
