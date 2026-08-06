<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {save} from "@tauri-apps/plugin-dialog";
  import {StudentCommand} from "$commands";
  import {overlayController} from "$controllers/overlayController";
  import {clickOutside, updatePosition} from "$actions";

  let {selected = $bindable(), anchor = $bindable()} = $props<{
    selected: Set<bigint>;
    anchor: HTMLElement | null;
  }>();
  let isVisible = $state(false);
  let isExporting = $state(false);
  let popoverStyle = $state("");
  let errorMsg = $state("");

  function open() {
    errorMsg = "";
    popoverStyle = updatePosition(anchor);
    isVisible = true;
  }

  function close() {
    isVisible = false;
  }

  async function doExport(ids: bigint[]) {
    if (isExporting || ids.length == 0) return;
    // 弹出保存对话框：默认文件名可修改，格式 xlsx
    const path = await save({
      defaultPath: "学生名单.xlsx",
      filters: [{name: "Excel", extensions: ["xlsx"]}],
    });
    if (!path) return; // 用户取消保存
    try {
      isExporting = true;
      await StudentCommand.expose(path, ids);
      close();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      isExporting = false;
    }
  }

  function exportAll() {
    void doExport(studentStore.students.map((s) => s.id));
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
  <div onclick={close}></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="popup"
    style={popoverStyle}
    use:clickOutside={{ callback: close, exclude: anchor}}
  >
    <h3>导出学生</h3>
    <span class="field-label">选择导出范围，然后指定保存位置（.xlsx）</span>

    <button
      type="button"
      class="export-option"
      onclick={exportAll}
      disabled={isExporting || studentStore.isLoading || studentStore.students.length == 0}
    >
      <span class="option-title">导出全部</span>
      <span class="option-desc">共 {studentStore.students.length} 名学生</span>
    </button>

    <button
      type="button"
      class="export-option"
      onclick={exportSelected}
      disabled={isExporting || selected.size == 0}
    >
      <span class="option-title">导出选中</span>
      <span class="option-desc">已选 {selected.size} 名学生</span>
    </button>

    {#if errorMsg}
      <div class="export-error">导出失败：{errorMsg}</div>
    {/if}
    {#if isExporting}
      <div class="export-status">正在导出...</div>
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

<style>
  .export-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-xxs);
    width: 100%;
    padding: var(--space-xs) var(--space-sm);
    border: var(--border-size-xs) solid var(--border-color-4);
    border-radius: var(--radius-sm);
    background: var(--color-background);
    cursor: pointer;
    text-align: left;
  }

  .export-option:hover:not(:disabled) {
    background: var(--color-hover);
  }

  .export-option:disabled {
    background: var(--color-disabled);
    cursor: not-allowed;
  }

  .option-title {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
    color: var(--text-color-secondary);
  }

  .option-desc {
    font-size: var(--font-size-xs);
    color: var(--text-color-content);
  }

  .export-error {
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    background: var(--color-page);
    color: var(--color-error);
    font-size: var(--font-size-xs);
  }

  .export-status {
    font-size: var(--font-size-xs);
    color: var(--text-color-content);
  }
</style>
