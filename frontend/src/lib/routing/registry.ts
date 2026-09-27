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
 * 三个方法：`show`（呈现这个位置）、`hide`（收起）、`isShown`（报告可见性）。
 * `hide` 不能省：**关闭不是导航** —— 不能要求用户点"取消"也走一次导航，所以它必须待在这里。
 *
 * ## payload
 *
 * `payload` 是**一次性命令参数**（可 JSON 序列化、不可执行），只有目标 owner 认识它。
 * 需要页面状态的载荷（例如"编辑这几个学生"）由**页面**用 `interpret` 注册解释器，
 * 在 `show` 之前把载荷翻译成本页状态；弹窗因此不必认识载荷，只管可见性。
 */
export type EntryHandler = {
  show: (payload?: unknown) => void;
  hide?: () => void;
  isShown?: () => boolean;
};

export type EntryRegistry<Id extends string = string> = {
  register(id: Id, handler: EntryHandler): void;
  /** 注册载荷解释器：在 `show` 之前执行（由拥有页面状态的页面注册） */
  interpret(id: Id, translate: (payload?: unknown) => void): void;
  unregister(id: Id): void;
  show(id: Id, payload?: unknown): void;
  hide(id: Id): void;
  isShown(id: Id): boolean;
  has(id: Id): boolean;
  hideAll(): void;
  /**
   * 已注册的名字。
   *
   * 刻意返回 `string[]` 而非 `Id[]`：`Id` 只出现在**入参**位置，宽表因此可以直接当作
   * 窄表使用（`App.svelte` 的窗口级表传给各页收窄的 props 时不必转型），
   * 而"本页只能打开本页子树的入口"仍由 props 上的 `Id` 在编译期保证。
   */
  names(): string[];
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
    show(id, payload) {
      interpreters.get(id)?.(payload);
      handlers.get(id)?.show(payload);
    },
    hide(id) {
      handlers.get(id)?.hide?.();
    },
    isShown(id) {
      return handlers.get(id)?.isShown?.() ?? false;
    },
    has(id) {
      return handlers.has(id);
    },
    hideAll() {
      handlers.forEach((handler) => handler.hide?.());
    },
    names() {
      return Array.from(handlers.keys());
    },
  };
}
