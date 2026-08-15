<script lang="ts">
  import MarkdownIt from "markdown-it";
  import hljs from "highlight.js";
  import DOMPurify from "dompurify";
  import "highlight.js/styles/github.css";
  import {error} from "@fltsci/tauri-plugin-tracing";

  let {markdown = ""}: { markdown?: string } = $props();

  const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: true,
    highlight: (str, lang) => {
      if (lang && hljs.getLanguage(lang)) {
        try {
          return hljs.highlight(str, {language: lang}).value;
        } catch (e) {
          error(e.message);
          return "";
        }
      }
      try {
        return hljs.highlightAuto(str).value;
      } catch (e) {
        error(e.message);
        return "";
      }
    },
  });

  const rawHtml = $derived(md.render(markdown));
  const safeHtml = $derived(DOMPurify.sanitize(rawHtml, {
    USE_PROFILES: {html: true},
    ADD_ATTR: ["target"],
  }));
</script>

<div class="markdown-body" style="background: transparent;">
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
