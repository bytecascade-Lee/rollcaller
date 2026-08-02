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
      popoverStyle = "position: fixed; top: 80px; left: 40px;";
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
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
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
  <div class="export-backdrop" onclick={close}></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="export-popover" style={popoverStyle} onclick={(e) => e.stopPropagation()}>
    <h3>导出学生</h3>
    <p class="export-hint">选择导出范围，然后指定保存位置（.xlsx）</p>

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

    <div class="export-actions">
      <button type="button" class="btn-secondary" onclick={close} disabled={isExporting}>取消</button>
    </div>
  </div>
{/if}

<style>
  .export-backdrop {
    position: fixed;
    inset: 0;
    z-index: 998;
    background: transparent;
  }

  .export-popover {
    position: fixed;
    z-index: 999;
    background: #fff;
    border: 1px solid #dee2e6;
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    padding: 14px;
    box-sizing: border-box;
    min-width: 220px;
  }

  .export-popover h3 {
    margin: 0 0 4px 0;
    font-size: 14px;
  }

  .export-hint {
    margin: 0 0 10px 0;
    font-size: 12px;
    color: #6c757d;
  }

  .export-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 8px 10px;
    margin-bottom: 6px;
    border: 1px solid #ced4da;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    text-align: left;
  }

  .export-option:hover:not(:disabled) {
    background: #f1f3f5;
  }

  .export-option:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .option-title {
    font-size: 13px;
    font-weight: 600;
  }

  .option-desc {
    font-size: 12px;
    color: #6c757d;
  }

  .export-error {
    margin-top: 6px;
    padding: 6px 8px;
    border-radius: 4px;
    background: #fdecea;
    color: #c0392b;
    font-size: 12px;
  }

  .export-status {
    margin-top: 6px;
    font-size: 12px;
    color: #6c757d;
  }

  .export-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
  }
</style>
