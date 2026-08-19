<script lang="ts">
  import MarkdownIt from "markdown-it";
  import hljs from "highlight.js";
  import DOMPurify from "dompurify";
  import "highlight.js/styles/github.css";
  import {error} from "@fltsci/tauri-plugin-tracing";
  import {openUrl} from "@tauri-apps/plugin-opener";

  let {markdown = "", onnavigate}: { markdown?: string; onnavigate?: (id: string) => void } = $props();

  type LinkAction = { kind: "docs"; id: string } | { kind: "external" } | null;

  /** 渲染期链接分类：文档链接 / 特殊文档链接 / 外部链接 / 其他。不改动源 md。 */
  function classifyLink(href: string): LinkAction {
    if (/^https?:\/\//i.test(href)) return {kind: "external"};
    //* README.md / README-en-US.md / CHANGELOG.md / RELEASE_NOTES.md / LICENSE
    const special = href.match(/(README(?:-en-US)?|CHANGELOG|RELEASE_NOTES|LICENSE)(?:\.md)?$/i);
    if (special) return {kind: "docs", id: special[0]};
    //* ../<id>/<id>-zh-CN.md → 提取目录名作为文档 id
    //* 目录名与文件名前缀必须一致，语言后缀（-zh-CN、-en-US等）可选
    const doc = href.match(/\.\.\/([^/]+)\/\1(?:-[a-z]{2}-[A-Z]{2})?\.md$/i);
    if (doc) return {kind: "docs", id: doc[1]};
    return null;
  }

  const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: true,
    highlight: (str, lang) => {
      if (lang && hljs.getLanguage(lang)) {
        try {
          return hljs.highlight(str, {language: lang}).value;
        } catch (e) {
          error(e instanceof Error ? e.message : String(e));
          return "";
        }
      }
      try {
        return hljs.highlightAuto(str).value;
      } catch (e) {
        error(e instanceof Error ? e.message : String(e));
        return "";
      }
    },
  });

  const defaultLinkOpen = md.renderer.rules.link_open ??
    ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));

  // 渲染期注入 data-*：docs 链接改 href=# 并带 data-id；external 保留 href 只标记 action
  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const href = token.attrGet("href") ?? "";
    const action = classifyLink(href);
    if (action?.kind === "docs") {
      token.attrSet("href", "#");
      token.attrSet("data-action", "docs");
      token.attrSet("data-id", action.id);
    } else if (action?.kind === "external") {
      token.attrSet("data-action", "external");
    }
    return defaultLinkOpen(tokens, idx, options, env, self);
  };

  const rawHtml = $derived(md.render(markdown));
  const safeHtml = $derived(DOMPurify.sanitize(rawHtml, {
    USE_PROFILES: {html: true},
    ADD_ATTR: ["target"],
  }));

  /** 点击只做调度：读 data-action 执行，不参与链接识别 */
  function handleClick(event: MouseEvent) {
    const anchor = (event.target as Element).closest("a");
    if (!anchor) return;
    const action = anchor.dataset.action;
    if (!action) return; // 无/未知 action → 忽略，不拦截默认行为
    event.preventDefault();
    if (action === "docs") {
      const id = anchor.dataset.id;
      if (id) onnavigate?.(id);
    } else if (action === "external") {
      const href = anchor.getAttribute("href");
      if (href) openUrl(href).catch((e) => error(e instanceof Error ? e.message : String(e)));
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="markdown-body" onclick={handleClick} style="background: transparent;">
  {@html safeHtml}
</div>

<style>
  .markdown-body {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-sm);
    line-height: 1.5;
    color: var(--text-color-primary);
    padding-right: var(--space-sm);
    background: var(--color-page);
    overflow-wrap: break-word;
    user-select: none;
  }

  .markdown-body :global(h1) {
    font-size: 1.6rem;
    padding-top: 0;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h2) {
    font-size: 1.35rem;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h3) {
    font-size: 1.15rem;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h4) {
    font-size: 1rem;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(p) {
    margin: 0 0 1rem;
  }

  .markdown-body :global(a) {
    color: var(--color-primary);
    text-decoration: none;
  }

  .markdown-body :global(a:hover) {
    text-decoration: underline;
  }

  .markdown-body :global(ul),
  .markdown-body :global(ol) {
    margin: 0 0 1rem 1.8rem;
    padding-left: 0;
  }

  .markdown-body :global(li) {
    margin-bottom: 0.25rem;
  }

  .markdown-body :global(li > ul),
  .markdown-body :global(li > ol) {
    margin-bottom: 0;
  }

  .markdown-body :global(blockquote) {
    margin: 0 0 1rem;
    padding: 0 1rem;
    border-left: 4px solid var(--border-color-3);
    color: var(--text-color-secondary);
  }

  .markdown-body :global(pre) {
    padding: var(--space-xs);
    border-radius: var(--radius-md);
    background: var(--color-card);
    white-space: pre-wrap;
    word-break: break-word;
    overflow-wrap: break-word;
  }

  .markdown-body :global(pre code) {
    font-family: var(--font-family-mono);
    font-size: 0.8rem;
    line-height: 1.6;
    padding: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .markdown-body :global(p code),
  .markdown-body :global(li code) {
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-md);
    background: var(--color-card);
    font-family: var(--font-family-mono);
    font-size: 0.8rem;
  }

  .markdown-body :global(table) {
    border-collapse: collapse;
    margin: 0 0 1rem;
    border-radius: var(--radius-md);
    width: 100%;
  }

  .markdown-body :global(th),
  .markdown-body :global(td) {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color-4);
    text-align: left;
  }

  .markdown-body :global(th li code),
  .markdown-body :global(td li code) {
    font-family: var(--font-family-mono);
  }

  .markdown-body :global(th) {
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(tr:nth-child(even)) {
    background: var(--color-page);
  }

  .markdown-body :global(hr) {
    border: 0;
    border-top: 2px solid var(--border-color-2);
    margin: 1.5rem 0;
  }

  .markdown-body :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--radius-sm);
  }

  .markdown-body :global(strong) {
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(em) {
    font-style: italic;
  }
</style>
