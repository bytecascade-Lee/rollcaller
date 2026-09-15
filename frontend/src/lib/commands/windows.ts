/**
 * 窗口命令（薄壳）：只留**没有地址语义**的窗口操作。
 *
 * "打开某处"一律走 `nav(action)`——打开帮助窗口就是 `nav(HelpEntries.overview)`，
 * 建窗 / 唤到前台 / 投递载荷由后端统一处理，所以不再需要
 * `windows_help_open` 这类"只能开到默认页、开不到指定位置"的专用命令。
 *
 * 已删除三个死封装：`hideHelpWindow` / `closeHelpWindow` / `destroyHelpWindow`——
 * 它们对应的 `windows_help_hide|close|destroy` 在后端 `generate_handler!` 里早已不存在
 * （`cmd/windows.rs` 只有 `list` / `app_open` / `help_open`），调用即抛错，属历史遗留。
 */
import {invoke} from "@tauri-apps/api/core";

/** 打开（或唤出）主窗口：帮助窗口标题栏的"主界面"按钮用 */
export async function openAppWindow() {
  await invoke<void>("windows_app_open", {});
}
