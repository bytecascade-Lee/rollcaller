<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {save} from "@tauri-apps/plugin-dialog";
  import {StudentCommand} from "$commands";
  import {overlayController} from "$controllers/popupController";
  import {clickOutside, updatePosition} from "$actions";
  import type {StudentTable} from "$types";
  import {Result} from "$types";

  let {selected = $bindable(), display = $bindable(), anchor = $bindable()} = $props<{
    selected: Set<bigint>;
    display: StudentTable[];
    anchor: HTMLElement | null;
  }>();
  let isVisible = $state(false);
  let isExporting = $state(false);
  let popoverStyle = $state("");
  let message = $state("");
  let result = $state<Result>(Result.None);

  function open() {
    popoverStyle = updatePosition(anchor);
    if (selected.size == 0 && studentStore.students.length == display.length) {
      exportAll();
      return;
    }
    message = "";
    result = Result.None;
    isVisible = true;
  }

  function close() {
    isVisible = false;
  }

  async function doExport(ids: bigint[]) {
    if (isExporting || ids.length == 0) return;
    // 弹出保存对话框：默认文件名可修改，格式 xlsx
    const path = await save({
      defaultPath: `学生名单-${Date.now()}.xlsx`,
      filters: [{name: "Excel", extensions: ["xlsx"]}],
    });
    if (!path) return;
    try {
      result = Result.Doing;
      await StudentCommand.expose(path, ids);
      result = Result.Success;
      setTimeout(() => close(), 2500);
    } catch (e) {
      result = Result.Error;
      message = String(e);
    } finally {
      isVisible = true;
      isExporting = false;
    }
  }

  function exportAll() {
    void doExport(studentStore.students.map((s) => s.id));
  }

  function exportDisplay() {
    void doExport(display.map((student: { id: bigint; }) => student.id))
  }

  function exportSelected() {
    void doExport(Array.from(selected));
  }

  $effect(() => {
    overlayController.register("StudentExport", {
      open: open,
      close: close,
      isVisible: () => isVisible,
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="popup"
    style={popoverStyle}
    use:clickOutside={{ callback: close, exclude: anchor}}
  >
    <h3 class="text-title">导出学生</h3>
    <span class="text-content">选择导出范围，然后指定保存位置（.xlsx）</span>

    <div
      class="card"
      onclick={exportAll}
    >
      <span class="text-subtitle">导出全部</span>
      <span class="text-content">共 {studentStore.students.length} 名学生</span>
    </div>

    {#if studentStore.students.length != display.length}
      <div
        class="card"
        onclick={exportDisplay}
      >
        <span class="text-subtitle">导出筛选后的</span>
        <span class="text-content">共 {display.length} 名学生</span>
      </div>
    {/if}

    {#if selected.size > 0}
      <div
        class="card"
        onclick={exportSelected}
      >
        <span class="text-subtitle">导出选中</span>
        <span class="text-content">已选 {selected.size} 名学生</span>
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
        onclick={close}
        disabled={isExporting}
      >
        取消
      </button>
    </div>
  </div>
{/if}
