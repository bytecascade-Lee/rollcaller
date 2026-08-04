<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {save} from "@tauri-apps/plugin-dialog";
  import {StudentCommand} from "$commands";
  import {overlayController} from "$controllers/overlayController";

  let {selected = $bindable()} = $props<{ selected: Set<bigint> }>()
  let isVisible = $state(false);
  let isExporting = $state(false);
  let popoverStyle = $state("");
  let errorMsg = $state("");

  /** 定位到工具栏下方（导出按钮所在区域），不依赖页面改动 */
  function updatePosition() {
    const toolbars = document.querySelectorAll(".toolbar");
    const toolbar = toolbars[toolbars.length - 1];
    if (toolbar) {
      const rect = toolbar.getBoundingClientRect();
      popoverStyle = `position: fixed; top: ${rect.bottom + 6}px; left: ${rect.left}px;`;
    } else {
      popoverStyle = `position: fixed; top: var(--size-9); left: var(--size-9);`;
    }
  }

  function open() {
    errorMsg = "";
    updatePosition();
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
  <div class="popup"
       style={popoverStyle}
       onclick={(e) => e.stopPropagation()}
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
    border: var(--border-size-xs) solid var(--border-color-regular);
    border-radius: var(--radius-sm);
    background: var(--color-page);
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
