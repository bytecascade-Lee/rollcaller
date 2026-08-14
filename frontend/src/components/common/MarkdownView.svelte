<script lang="ts">
  import MarkdownIt from "markdown-it";
  import hljs from "highlight.js";
  import DOMPurify from "dompurify";
  import "highlight.js/styles/github.css";
  import {error} from "@fltsci/tauri-plugin-tracing";

  let { markdown = "" }: { markdown?: string } = $props();

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

<div class="markdown-body">
  {@html safeHtml}
</div>

<style>
  .markdown-body {
    padding: 1.5rem 2rem;
    font-family: var(--font-family-sans);
    font-size: var(--font-size-lg);
    line-height: 1.7;
    color: var(--text-color-primary);
    background: var(--color-card);
    max-width: 900px;
    margin: 0 auto;
  }

  .markdown-body :global(h1) {
    font-size: 2.2rem;
    font-weight: var(--font-weight-bold);
    margin: 1.8rem 0 1rem;
    padding-bottom: 0.3rem;
    border-bottom: 2px solid var(--border-color-2);
  }

  .markdown-body :global(h2) {
    font-size: 1.6rem;
    font-weight: var(--font-weight-bold);
    margin: 1.5rem 0 0.75rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--border-color-2);
  }

  .markdown-body :global(h3) {
    font-size: 1.3rem;
    font-weight: var(--font-weight-bold);
    margin: 1.2rem 0 0.6rem;
  }

  .markdown-body :global(h4) {
    font-size: 1.1rem;
    font-weight: var(--font-weight-bold);
    margin: 1rem 0 0.5rem;
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
    padding: 1rem 1.2rem;
    border-radius: var(--radius-md);
    overflow-x: auto;
    margin: 0 0 1rem;
    background: var(--color-hover);
  }

  .markdown-body :global(pre code) {
    font-family: var(--font-family-mono);
    font-size: 0.9rem;
    line-height: 1.6;
    background: transparent;
    padding: 0;
  }

  .markdown-body :global(p code),
  .markdown-body :global(li code) {
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-xxs);
    background: var(--color-hover);
    font-family: var(--font-family-mono);
    font-size: 0.9rem;
  }

  .markdown-body :global(table) {
    border-collapse: collapse;
    margin: 0 0 1rem;
    width: 100%;
  }

  .markdown-body :global(th),
  .markdown-body :global(td) {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color-2);
    text-align: left;
  }

  .markdown-body :global(th) {
    background: var(--color-hover);
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
