<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {save} from "@tauri-apps/plugin-dialog";
  import {StudentCommand} from "$commands";
  import type {EntryRegistry} from "$routing/registry";
  import type {StudentOverlayName} from "$types";
  import {Result} from "$types";
  import type {AppEntryPayload} from "$routing/app/entries.generated";
  import {AppEntry} from "$routing/app/entries.generated";
  import {clickOutside, findAnchor, POPUP_CENTERED, updatePosition} from "$actions";
  import {appNavigate} from "$routing/app/navigate";

  let {overlays}: { overlays: EntryRegistry<StudentOverlayName, AppEntryPayload> } = $props();

  let isVisible = $state(false);
  let isExporting = $state(false);
  let popoverStyle = $state("");
  let message = $state("");
  let result = $state<Result>(Result.None);
  /** 触发元素，查不到就居中，同时兼作 clickOutside 的排除项 */
  let anchor = $state<HTMLElement | null>(null);

  let filtered = $state<number[]>([]);
  let selected = $state<number[]>([]);

  // [TODO]：此处获取 payload 类型时应传递 AppEntry.exportStudent，而不是原始字符串
  function open(payload: AppEntryPayload["export-student"] | undefined) {
    if (!payload || !Array.isArray(payload.filtered) || !Array.isArray(payload.selected)) return;
    filtered = payload.filtered;
    selected = payload.selected;
    message = "";
    result = Result.None;
    anchor = findAnchor(AppEntry.exportStudent);
    popoverStyle = anchor ? updatePosition(anchor) : POPUP_CENTERED;
    isVisible = true;
  }

  function close() {
    isVisible = false;
  }

  function requestClose() {
    appNavigate.leave(AppEntry.exportStudent);
  }

  async function doExport(ids: readonly number[]) {
    if (isExporting || ids.length == 0) return;
    // 弹出保存对话框：默认文件名可修改，格式 xlsx
    const path = await save({
      defaultPath: `学生名单-${Date.now()}.xlsx`,
      filters: [{name: "Excel", extensions: ["xlsx"]}],
    });
    if (!path) return;
    try {
      result = Result.Doing;
      await StudentCommand.expose(path, ids as unknown as bigint[]);
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
    void doExport(studentStore.students.map((student) => Number(student.id)));
  }

  function exportFiltered() {
    void doExport(filtered);
  }

  function exportSelected() {
    void doExport(selected);
  }

  $effect(() => {
    overlays.register(AppEntry.exportStudent, {
      open,
      close,
    })
    return () => overlays.unregister(AppEntry.exportStudent);
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="popup"
    style={popoverStyle}
    use:clickOutside={{ callback: requestClose, exclude: anchor ?? undefined}}
  >
    <h3 class="text-title">导出学生</h3>
    <span class="text-content">选择导出范围</span>

    <div
      class="card"
      onclick={exportAll}
    >
      <span class="text-subtitle">导出全部</span>
      <span class="text-content">共 {studentStore.students.length} 名学生</span>
    </div>

    {#if selected.length > 0 && studentStore.students.length != selected.length}
      <div
        class="card"
        onclick={exportSelected}
      >
        <span class="text-subtitle">导出被选中的</span>
        <span class="text-content">共 {selected.length} 名学生</span>
      </div>
    {/if}

    {#if filtered.length > 0 && studentStore.students.length != filtered.length}
      <div
        class="card"
        onclick={exportFiltered}
      >
        <span class="text-subtitle">导出筛选后的</span>
        <span class="text-content">共 {filtered.length} 名学生</span>
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
      <button
        type="button"
        class="button"
        onclick={requestClose}
        disabled={isExporting}
      >
        取消
      </button>
    </div>
  </div>
{/if}
