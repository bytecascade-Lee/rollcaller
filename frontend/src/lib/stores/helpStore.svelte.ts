import {HelpCommand} from "$commands";
import metaData from "$resources/help/meta.json";

/** 特殊节点走独立命令，其余叶子走 help_load_markdown(id)。 */
const SPECIAL_LOADERS: Record<string, () => Promise<string>> = {
  "README.md": HelpCommand.readme,
  "LICENSE": HelpCommand.license,
  "CHANGELOG.md": HelpCommand.changelog,
  "RELEASE_NOTES.md": HelpCommand.releaseNotes,
};

const MAX_RESULTS = 30;

export type HelpSearchResult = {
  /** 唯一标识（同一文档多处命中时各不相同），供列表 key 使用 */
  key: string;
  id: string;
  title: string;
  snippet: string;
};

class HelpStore {
  content = $state("");
  /** 全文索引构建中（首次搜索时置 true）。 */
  indexing = $state(false);

  /** 搜索结果选中后的跳转回调，由 Help 页面注册（复用导航 + 树高亮）。 */
  navigate: ((id: string) => void) | null = null;

  #cache = new Map<string, string>();
  #clean = new Map<string, string>();
  #titles = new Map<string, string>();
  #indexPromise: Promise<void> | null = null;

  /** 按 id 加载内容：命中缓存直接返回，否则经分发表拉取并缓存。 */
  async load(id: string) {
    try {
      if (!this.#cache.has(id)) {
        const loader = SPECIAL_LOADERS[id] ?? (() => HelpCommand.markdown(id));
        this.#cache.set(id, await loader());
      }
      this.content = this.#cache.get(id)!;
    } catch (e) {
      // 无对应 md 文件（或加载失败）→ 清空内容，右侧展示占位提示
      this.#cache.delete(id);
      this.content = "";
    }
  }

  /** 预加载全部叶子文档并建立搜索索引；幂等，进程内只执行一次。 */
  ensureIndexed(): Promise<void> {
    if (!this.#indexPromise) {
      this.indexing = true;
      this.#indexPromise = this.#buildIndex().finally(() => {
        this.indexing = false;
      });
    }
    return this.#indexPromise;
  }

  async #buildIndex() {
    const nodes = metaData.nodes as Record<string, { title: string }>;
    await Promise.all(
      this.#leafIds().map(async (id) => {
        this.#titles.set(id, nodes[id]?.title ?? id);
        try {
          const loader = SPECIAL_LOADERS[id] ?? (() => HelpCommand.markdown(id));
          const md = await loader();
          this.#cache.set(id, md);
          this.#clean.set(id, this.#cleanText(md));
        } catch {
          // 文档无内容：仅参与标题命中，不中断索引
        }
      })
    );
  }

  /** 在已建索引上搜索：标题命中整体置顶（保持文档顺序），内容命中取纯文本片段。 */
  search(query: string): HelpSearchResult[] {
    if (!this.#indexPromise || !query.trim()) return [];
    const lower = query.toLowerCase();
    const found: HelpSearchResult[] = [];
    const titleMatched = new Set<string>();

    // 第一轮：标题命中，保持文档顺序，整体排最前
    for (const id of this.#leafIds()) {
      const title = this.#titles.get(id) ?? id;
      if (title.toLowerCase().includes(lower)) {
        titleMatched.add(id);
        found.push({key: `${id}#title`, id, title, snippet: ""});
      }
    }

    // 第二轮：内容命中（已标题命中的文档不再重复），同文档每处一条
    for (const id of this.#leafIds()) {
      if (titleMatched.has(id)) continue;
      const title = this.#titles.get(id) ?? id;
      const content = this.#clean.get(id) ?? "";
      const contentLower = content.toLowerCase();
      let searchFrom = 0;
      let hits = 0;
      while (found.length < MAX_RESULTS * 100 && hits < 300) {
        const idx = contentLower.indexOf(lower, searchFrom);
        if (idx === -1) break;
        hits += 1;
        found.push({
          key: `${id}#${idx}`,
          id,
          title,
          snippet: this.#buildSnippet(content, idx, query.length),
        });
        searchFrom = idx + query.length;
      }
    }
    return found;
  }

  /** 截取命中位置上下文片段（文本已清理，无需处理符号）。 */
  #buildSnippet(text: string, idx: number, queryLen: number): string {
    const start = Math.max(0, idx - 12);
    const end = Math.min(text.length, idx + queryLen + 24);
    return (
      (start > 0 ? "…" : "") +
      text.slice(start, end).trim() +
      (end < text.length ? "…" : "")
    );
  }

  /** 去除 markdown 符号得到纯文本：先清理再命中，链接只保留文字、URL 不参与搜索。 */
  #cleanText(md: string): string {
    return md
      // 导航锚点注解行（[//]: # (@section: x) / @link），避免污染搜索命中
      .replace(/^\s*\[\/\/\]:\s*#\s*\(@(?:section|link):[^)]*\)\s*$/gm, "")
      // 代码块围栏整段移除
      .replace(/```[\s\S]*?```/g, " ")
      // 行内代码
      .replace(/`([^`]*)`/g, "$1")
      // 图片 → 替代文本
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
      // 链接 → 链接文字（URL 不参与命中）
      .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
      // 行首标题符号、引用、列表符号、有序列表、分割线
      .replace(/^\s{0,3}#{1,6}\s+/gm, "")
      .replace(/^\s{0,3}>\s?/gm, "")
      .replace(/^\s*[-*+]\s+/gm, "")
      .replace(/^\s*\d+[.、)]\s+/gm, "")
      .replace(/^\s*(?:-{3,}|\*{3,})\s*$/gm, "")
      // 行内强调/删除线
      .replace(/\*\*([^*]*)\*\*/g, "$1")
      .replace(/\*([^*]*)\*/g, "$1")
      .replace(/~~([^~]*)~~/g, "$1")
      // HTML 标签（如 <kbd>）
      .replace(/<[^>]+>/g, "")
      // 折叠空白为单空格
      .replace(/\s+/g, " ");
  }

  #leafIds(): string[] {
    const nodes = metaData.nodes as Record<string, unknown>;
    const order = metaData.order as Record<string, string[]>;
    const hasChildren = new Set(Object.keys(order));
    return Object.keys(nodes).filter((id) => !hasChildren.has(id));
  }
}

export const helpStore = new HelpStore();
