#!/usr/bin/env python3
# -*- coding: UTF-8 -*-

"""
首页 HTML
"""

from collections.abc import Iterable
from html import escape

from serve.source import AssetRow, CheckRow, EndpointRow, IndexView

_STYLE = """
         :root {
             color-scheme: light dark;
             --fg: #1b1b1b;
             --dim: #6b7280;
             --line: #e5e7eb;
             --ok: #1a7f37;
             --bad: #b42318;
             --mono: "Maple Mono NF CN", "Cascadia Mono", Consolas, monospace;
         }

         @media (prefers-color-scheme: dark) {
             :root {
                 --fg: #e6e6e6;
                 --dim: #9aa0a6;
                 --line: #33383d;
                 --ok: #4ac26b;
                 --bad: #ff7b72;
             }
         }

         * {
             box-sizing: border-box;
         }

         body {
             max-width: 1000px;
             margin: 0 auto;
             padding: 40px 28px 64px;
             color: var(--fg);
             font: 14px/1.65 "LXGW WenKai GB Screen", system-ui, -apple-system, "Segoe UI", "Microsoft YaHei";
         }

         header {
             display: flex;
             justify-content: space-between;
             align-items: baseline;
             gap: 24px;
             padding-bottom: 14px;
             border-bottom: 1px solid var(--line);
         }

         h1 {
             margin: 0;
             font-size: 18px;
             font-weight: 600;
         }

         h2 {
             margin: 0 0 4px;
             font-size: 13px;
             font-weight: 600;
             color: var(--dim);
         }

         p {
             margin: 0;
         }

         a {
             color: inherit;
         }

         dl {
             display: grid;
             grid-template-columns: 72px minmax(0, 1fr);
             gap: 4px 16px;
             margin: 16px 0 0;
             font-size: 13px;
         }

         dt {
             color: var(--dim);
         }

         dd {
             margin: 0;
             overflow-wrap: anywhere;
         }

         .mono {
             font-family: var(--mono);
             font-size: 13px;
         }

         .dim {
             color: var(--dim);
         }

         section {
             margin-top: 28px;
         }

         .row {
             display: flex;
             justify-content: space-between;
             gap: 24px;
             padding: 7px 0;
             border-bottom: 1px solid var(--line);
         }

         .row a {
             overflow: hidden;
             text-overflow: ellipsis;
             white-space: nowrap;
             text-decoration: none;
         }

         .row a:hover {
             text-decoration: underline;
         }

         .row span {
             flex: 0 0 auto;
             color: var(--dim);
             font-size: 13px;
         }

         .ok {
             color: var(--ok);
         }

         .bad {
             color: var(--bad);
         }

         footer {
             margin-top: 36px;
             padding-top: 14px;
             border-top: 1px solid var(--line);
             color: var(--dim);
             font-size: 12px;
         } \
         """


def render(view: IndexView) -> str:
    """渲染整个页面"""
    return f"""
<!doctype html>
<html lang="zh-CN">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <title>rollcaller {escape(view.version)} 本地更新源</title>
        <style>{_STYLE}</style>
    </head>
    <body>
        <header>
            <div>
                <h1>本地更新源</h1>
                <p class="mono dim">{escape(view.origin)}</p>
            </div>
            <span class="mono dim">
                {escape(view.version)}
            </span>
        </header>
        <dl>
            <dt>目录</dt><dd class="mono">{escape(str(view.directory))}</dd>
            <dt>清单</dt><dd>{escape(view.published)} 发布 · severity {escape(view.severity)}</dd>
            <dt>限速</dt><dd>{escape(view.throttle)}</dd>
            <dt>自检</dt><dd>{_checks(view.checks)}</dd>
        </dl>
        {_section("后端 URL", view.endpoints)}
        {_section("实际产物", view.assets, empty="清单里没有载荷")}
        {_section("伴生文件", view.companions, empty="没有伴生文件")}
        {_section("兼容形态", view.compat)}
        <footer>执行 Ctrl+C 退出 · 限速只作用于产物直链，清单与索引全速 · 当前暂不支持 Range</footer>
    </body>
</html>
"""


def _checks(checks: Iterable[CheckRow]) -> str:
    return "<br>".join(_check(check) for check in checks)


def _check(check: CheckRow) -> str:
    state = "ok" if check.ok else "bad"
    return f'<span class="{state}">{escape(check.text)}</span>'


def _section(title: str, rows: Iterable[AssetRow | EndpointRow], *, empty: str | None = None) -> str:
    """一节 = 标题 + 若干行；没有行时按 empty 给一句空态，empty 为 None 则整节不出现"""
    items = list(rows)
    if items:
        body = "".join(_row(item) for item in items)
    elif empty is not None:
        body = f'<p class="dim">{escape(empty)}</p>'
    else:
        return ""
    return f"<section><h2>{escape(title)}</h2>{body}</section>"


def _row(row: AssetRow | EndpointRow) -> str:
    if isinstance(row, AssetRow):
        label, href, note = row.name, row.href, row.size_text
    else:
        label, href, note = row.path, row.path, f"{row.note} · {row.size_text}"
    return f"""
        <div class="row">
            <a class="mono" href="{escape(href, quote=True)}"
                title="{escape(label, quote=True)}">{escape(label)}
            </a>
            <span>{escape(note)}</span>
        </div>
        """
