/**
 * 帮助文档导航锚点注解（[//]: # (@section: x) / [//]: # (@link: x)）的预处理与 token 注入。
 *
 * 渲染管线：
 *   stripAnnotations(md)  逐行扫描、剥离注释行，产出干净 markdown 与注解元数据；
 *   attachAnnotations()   在 markdown-it core 阶段把 data-section 注入标题与链接 token。
 *
 * 注解语法（见 resources/help 下文档约定）：
 *   [//]: # (@section: kebab-name)   // 紧随其后的标题声明逻辑锚点
 *   [//]: # (@link: kebab-name)       // 紧随其后的行中第一个带 `#` 的链接声明跳转目标
 *
 * 行号锚定采用【原始行号】而非剥离后的行号：连续剥离多条注释时，
 * 目标块在清理后源码中的行号会整体上移，原始行号不受影响。
 */
import type {Token} from "markdown-it";

/** @link 注解指令（原始行号，0-based） */
type LinkDirective = {origLine: number; name: string};

export type HelpAnnotations = {
  /** 剥离注释行后的干净 markdown */
  clean: string;
  /** clean 行号 → 原始行号（仅保留行） */
  cleanToOrig: number[];
  /** 标题原始行号 → @section 锚点名 */
  sectionByOrigLine: Map<number, string>;
  /** 块起始原始行号 → 该块内按文档顺序对应的 @link 锚点名列表 */
  linkRunAtOrigStart: Map<number, string[]>;
};

/** [//]: # (@section: name) — 锚点名 kebab-case 小写英文 */
const SECTION_RE = /^[ \t]*\[\/\/\]:[ \t]*#\s*\(@section:[ \t]*([a-z0-9]+(?:-[a-z0-9]+)*)[ \t]*\)[ \t]*$/;
/** [//]: # (@link: name) */
const LINK_RE = /^[ \t]*\[\/\/\]:[ \t]*#\s*\(@link:[ \t]*([a-z0-9]+(?:-[a-z0-9]+)*)[ \t]*\)[ \t]*$/;
/** 形似锚点注解但语法不合法（作者笔误），剥离并告警，避免污染渲染 */
const MALFORMED_RE = /^[ \t]*\[\/\/\]:[ \t]*#\s*\(@(?:section|link):/;
const HEADING_RE = /^#{1,6}[ \t]+/;

/**
 * 剥离注解注释行，返回干净 markdown 与注入所需的元数据。
 * @section 附着紧随其后的标题（中间允许空行；夹有其他内容则视为无效）。
 * 连续行的 @link 组成一个 run，run 内名字按出现顺序对应目标块内带 `#` 的链接。
 */
export function stripAnnotations(md: string): HelpAnnotations {
  // 统一行尾：Windows 文档为 CRLF，裸 `\r` 会让行尾锚定正则失效
  const lines = md.replace(/\r\n?/g, "\n").split("\n");
  const cleanLines: string[] = [];
  const cleanToOrig: number[] = [];
  const sectionByOrigLine = new Map<number, string>();
  const linkDirectives: LinkDirective[] = [];
  let pendingSection: string | null = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    const section = line.match(SECTION_RE);
    if (section) {
      pendingSection = section[1];
      continue;
    }
    const link = line.match(LINK_RE);
    if (link) {
      linkDirectives.push({origLine: i, name: link[1]});
      continue;
    }
    if (MALFORMED_RE.test(line)) {
      console.warn(`[help-annotations] 无法识别的锚点注解（已剥离）: 第 ${i + 1} 行 "${line.trim()}"`);
      continue;
    }
    if (pendingSection !== null) {
      if (HEADING_RE.test(line)) {
        sectionByOrigLine.set(i, pendingSection);
      } else if (line.trim() !== "") {
        // @section 与标题之间夹了正文 → 注解失效
        pendingSection = null;
      }
    }
    cleanLines.push(line);
    cleanToOrig.push(i);
  }

  // 相邻行号的 @link 合并为 run；run 结束行 + 1 即目标块在原始源码中的起始行
  const linkRunAtOrigStart = new Map<number, string[]>();
  for (let i = 0; i < linkDirectives.length; ) {
    let j = i;
    while (j + 1 < linkDirectives.length && linkDirectives[j + 1].origLine === linkDirectives[j].origLine + 1) {
      j += 1;
    }
    linkRunAtOrigStart.set(linkDirectives[j].origLine + 1, linkDirectives.slice(i, j + 1).map((d) => d.name));
    i = j + 1;
  }

  return {clean: cleanLines.join("\n"), cleanToOrig, sectionByOrigLine, linkRunAtOrigStart};
}

/** 嵌套块容器：其 children 是块级 token，需要递归 */
const BLOCK_CONTAINERS: Record<string, true> = {
  bullet_list: true,
  ordered_list: true,
  blockquote: true,
  list_item: true,
};

/**
 * 在 markdown-it core 阶段向 token 注入 data-section：
 * - heading_open 按原始行号匹配 @section；
 * - 块内带 `#` 的 link_open 按文档顺序匹配 @link run（外层容器与内层段落
 *   起始行相同，用起始行去重避免重复匹配）。
 */
export function attachAnnotations(ann: HelpAnnotations, tokens: Token[]): void {
  const {cleanToOrig, sectionByOrigLine, linkRunAtOrigStart} = ann;
  let lastOrigStart = -1;
  let runNames: string[] = [];

  const walk = (list: Token[]) => {
    for (const t of list) {
      if (t.map) {
        const origStart = cleanToOrig[t.map[0]];
        if (origStart !== lastOrigStart) {
          lastOrigStart = origStart;
          runNames = linkRunAtOrigStart.get(origStart) ?? [];
        }
      }
      if (t.type === "heading_open" && t.map) {
        const name = sectionByOrigLine.get(cleanToOrig[t.map[0]]);
        if (name) t.attrSet("data-section", name);
      }
      if (t.type === "inline" && runNames.length > 0) {
        let k = 0;
        for (const c of t.children ?? []) {
          if (c.type !== "link_open") continue;
          const href = String(c.attrGet("href") ?? "");
          if (!href.includes("#")) continue;
          if (k < runNames.length) c.attrSet("data-section", runNames[k]);
          k += 1;
        }
        runNames = [];
      }
      if (BLOCK_CONTAINERS[t.type] && t.children) walk(t.children);
    }
  };
  walk(tokens);
}
