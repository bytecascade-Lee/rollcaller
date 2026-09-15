/**
 * 页面内的叠加层注册表。
 *
 * 与旧 `popupController` 的区别**只有作用域**：签名（名字 → 开关）没变，变的是一张表
 * 服务一个页面，而不是服务整个应用。旧的那张病在 `PopupName` 是跨全应用的 12 个名字
 * 的全局命名空间——"哪个页面拥有哪个弹窗"在类型上没有任何表达；收敛到页面之后，
 * 名字可以是 `"edit"` 这种本页唯一就够的短名，`Name` 由页面自己给出，编译期就拦得住
 * "这个页面根本没有这个弹窗"。
 *
 * `params` 的定位要说清：它是 **owner 与弹窗之间的私有约定，不是路由载荷**。
 * 页面先把路由载荷翻译成自己内部的状态（选中了谁、要开哪个弹窗），再按需把弹窗真正
 * 需要的那些值丢给 `open(params)`。路由层自始至终不认识 `params` 里面的东西。
 *
 * 哪些弹窗该进地址空间（可深链、可跨窗口投递）、哪些只在本页内部用，由"地址表达
 * 用户看到的什么，不表达用户做了什么"这条判据筛：带实体身份的界面态（编辑某人）
 * 进；纯过程（导入向导、各类确认框、导出）不进，继续只用 `open(name)`。
 */

/** 一个叠加层的最小契约：能开、能关，可选地报告可见性 */
export type Overlay = {
  /** `params` 是 owner 与弹窗的私有约定，路由层不解释 */
  open: (params?: unknown) => void;
  close: () => void;
  isVisible?: () => boolean;
};

export type OverlayRegistry<Name extends string = string> = {
  register(name: Name, overlay: Overlay): void;
  unregister(name: Name): void;
  open(name: Name, params?: unknown): void;
  close(name: Name): void;
  isOpen(name: Name): boolean;
  closeAll(): void;
  names(): Name[];
};

export function createOverlayRegistry<Name extends string = string>(): OverlayRegistry<Name> {
  const overlays = new Map<Name, Overlay>();
  return {
    register(name, overlay) {
      overlays.set(name, overlay);
    },
    unregister(name) {
      overlays.delete(name);
    },
    open(name, params) {
      overlays.get(name)?.open(params);
    },
    close(name) {
      overlays.get(name)?.close();
    },
    isOpen(name) {
      return overlays.get(name)?.isVisible?.() ?? false;
    },
    closeAll() {
      overlays.forEach((overlay) => overlay.close());
    },
    names() {
      return Array.from(overlays.keys());
    },
  };
}
