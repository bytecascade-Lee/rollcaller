/**
 * 帮助文档地址：`#/help/<doc>[/<anchor>]`。
 *
 * - `doc`    = 文档 id，即 resources/help 下的目录名（README.md 等根文档沿用原名）
 * - `anchor` = 标题上的 DOM id，源文写作 `## 标题 {#anchor}`
 */
import {parseRoute, type Route, routeOf} from "$routing/address";

/** kebab-case 锚点/目录名：小写字母数字，单连字符分隔 */
const KEBAB_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

/** 文档内相对链接：`../<id>/<id>[-lang].md[#frag]`（目录名与文件名前缀必须一致） */
const DOC_LINK_RE = /^\.\.\/([^/]+)\/\1(?:-[a-z]{2}-[A-Z]{2})?\.md(?:#(.*))?$/;

/** 直接指向根目录文档的链接：`.../README.md[#frag]` 等 */
const ROOT_DOC_LINK_RE = /(?:^|\/)(README(?:-en-US)?\.md|CHANGELOG\.md|RELEASE_NOTES\.md|LICENSE)(?:#(.*))?$/;

/** 带协议前缀（http: / mailto: / docs: …）或以 `#`、`/` 开头的 href：一律不当作文档链接 */
const NON_DOC_HREF_RE = /^(?:[a-z][a-z0-9+.-]*:|\/|#)/i;

export class HelpAddress {
  /**
   * 本域的处理者名，恰好是帮助窗口的窗口名。
   *
   * 用 `#/` 而不是大小写来区分"应用内路由"与"本文档原生锚点"（`#v0-8-1` 没有斜杠），
   * 所以域名统一小写并不损失一眼可辨性。
   */
  static readonly HOST = "help";

  /** 路由省略 doc 段时的默认文档 */
  static readonly DEFAULT_DOC = "overview";

  readonly doc: string;
  readonly anchor: string | null;

  private constructor(doc: string, anchor: string | null) {
    this.doc = doc;
    this.anchor = anchor;
  }

  /**
   * 应用代码构造（帮助按钮 / 搜索 / 导航树）。
   * 不做语法校验：调用点由类型与构建期校验器兜底，运行期任何偏差都表现为
   * "占位提示"或"滚到文档顶部"，不会静默失败。
   */
  static of(doc: string, anchor?: string | null): HelpAddress {
    if (!doc) throw new Error("HelpAddress.of: doc 不能为空");
    return new HelpAddress(doc, anchor ? anchor : null);
  }

  /**
   * 解析应用内路由。**只要 host 命中就一定返回地址**（绝不返回 null），
   * 因为放行意味着 webview 真的去导航 → 404。
   * 传入带或不带 `#` 均可（`location.hash` 与 href 都带 `#`）。
   */
  static parse(hash: string): HelpAddress | null {
    const parsed = parseRoute(hash);
    if (!parsed || parsed.host !== HelpAddress.HOST) return null;
    const [doc, anchor = ""] = parsed.segments;
    return new HelpAddress(doc || HelpAddress.DEFAULT_DOC, KEBAB_RE.test(anchor) ? anchor : null);
  }

  /**
   * 解析作者写在文档里的相对链接（渲染期改写用）。
   * 无上下文：同文档锚点（`#xxx`）与外部链接一律返回 null 交回浏览器原生处理，
   * 因此本函数不依赖"当前是哪篇文档"。
   */
  static fromDocLink(href: string): HelpAddress | null {
    if (!href || NON_DOC_HREF_RE.test(href)) return null;
    const doc = DOC_LINK_RE.exec(href) ?? null;
    if (doc) return new HelpAddress(doc[1], HelpAddress.#anchorOrNull(doc[2]));
    const root = ROOT_DOC_LINK_RE.exec(href);
    if (root) return new HelpAddress(root[1], HelpAddress.#anchorOrNull(root[2]));
    return null;
  }

  /** 应用内路由字符串；同一地址的字符串必然相同，可直接用于幂等比较 */
  toHash(): Route {
    return routeOf(HelpAddress.HOST, this.anchor ? [this.doc, this.anchor] : [this.doc]);
  }

  toString(): string {
    return this.toHash();
  }

  /** 非 kebab 的 fragment（如旧文档里的中文 slug）降级为"文档顶部" */
  static #anchorOrNull(fragment: string | undefined): string | null {
    return fragment && KEBAB_RE.test(fragment) ? fragment : null;
  }
}
