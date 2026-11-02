<script lang="ts">
  import {save} from "@tauri-apps/plugin-dialog";
  import {RecordCommand} from "$commands";
  import type {EntryRegistry} from "$routing/registry";
  import type {RecordOverlayName} from "$types";
  import {Result} from "$types";
  import type {AppEntryPayload} from "$routing/app/entries.generated";
  import {AppEntry} from "$routing/app/entries.generated";
  import {clickOutside, findAnchor, POPUP_CENTERED, updatePosition} from "$actions";
  import {recordStore} from "$stores/recordStore.svelte";
  import {appNavigate} from "$routing/app/navigate";

  let {overlays}: {overlays: EntryRegistry<RecordOverlayName, AppEntryPayload>} = $props();

  let isVisible = $state(false);
  let isExporting = $state(false);
  let popoverStyle = $state("");
  let message = $state("");
  let result = $state<Result>(Result.None);
  /** 触发元素（查不到就居中）；同时兼作 clickOutside 的排除项 */
  let anchorEl = $state<HTMLElement | null>(null);

  /** 载荷快照：筛选集与选中集都由页面给 —— 筛选条件是页面视图，从 store 推不出来 */
  let filtered = $state<number[]>([]);
  let selected = $state<number[]>([]);

  /** 请求关闭：地址回退到父级（关闭是导航，不是本地状态）；`close()` 只留给注册表执行真正的隐藏 */
  function requestClose() {
    appNavigate.leave(AppEntry.exportRecord);
  }

  /** 载荷为空（深链只有地址）⇒ 不开，降级为纯导航。形状直接取声明，不在此重写一遍 */
  function open(payload: AppEntryPayload["export-record"] | undefined) {
    if (!payload || !Array.isArray(payload.filtered) || !Array.isArray(payload.selected)) return;
    filtered = payload.filtered;
    selected = payload.selected;
    message = "";
    result = Result.None;
    anchorEl = findAnchor(AppEntry.exportRecord);
    popoverStyle = anchorEl ? updatePosition(anchorEl) : POPUP_CENTERED;
    isVisible = true;
  }

  function close() {
    isVisible = false;
  }

  async function doExport(ids: readonly number[]) {
    if (isExporting || ids.length == 0) return;
    // 弹出保存对话框：默认文件名可修改，格式 xlsx
    const path = await save({
      defaultPath: `历史记录-${Date.now()}.xlsx`,
      filters: [{name: "Excel", extensions: ["xlsx"]}],
    });
    if (!path) return;
    try {
      result = Result.Doing;
      // 命令签名是 ts-rs 对 i64 的投影，运行期就是同一批 number —— **不能**构造真 BigInt，
      // 否则 IPC 的 JSON.stringify 会抛 "Do not know how to serialize a BigInt"
      await RecordCommand.expose(path, ids as unknown as bigint[]);
      result = Result.Success;
      setTimeout(requestClose, 2500);
    } catch (e) {
      result = Result.Error;
      message = String(e);
    } finally {
      isVisible = true;
      isExporting = false;
    }
  }

  function exportAll() {
    void doExport(recordStore.records.map((record) => Number(record.id)));
  }

  function exportFiltered() {
    void doExport(filtered);
  }

  function exportSelected() {
    void doExport(selected);
  }

  $effect(() => {
    overlays.register(AppEntry.exportRecord, {
      open,
      close,
    });
    return () => overlays.unregister(AppEntry.exportRecord);
  });
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="popup" style={popoverStyle} use:clickOutside={{callback: requestClose, exclude: anchorEl ?? undefined}}>
    <h3 class="text-title">导出记录</h3>
    <span class="text-content">选择导出范围</span>

    <div class="card" onclick={exportAll}>
      <span class="text-subtitle">导出全部</span>
      <span class="text-content">共 {recordStore.records.length} 条记录</span>
    </div>

    {#if recordStore.records.length != filtered.length}
      <div class="card" onclick={exportFiltered}>
        <span class="text-subtitle">导出筛选后的</span>
        <span class="text-content">共 {filtered.length} 条记录</span>
      </div>
    {/if}

    {#if selected.length > 0}
      <div class="card" onclick={exportSelected}>
        <span class="text-subtitle">导出选中</span>
        <span class="text-content">已选 {selected.length} 条记录</span>
      </div>
    {/if}

    {#if result == Result.Error}
      <span class="text-subtitle error">导出失败</span>
      <span class="text-content error">{message}</span>
    {:else if result == Result.Success}
      <span class="text-subtitle success">导出成功</span>
    {:else if result == Result.Doing}
      <span class="text-subtitle info">正在导出...</span>
    {/if}

    <div class="button-group">
      <button type="button" class="button" onclick={requestClose} disabled={isExporting}> 取消 </button>
    </div>
  </div>
{/if}
