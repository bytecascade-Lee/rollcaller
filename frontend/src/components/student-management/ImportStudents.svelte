<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {ImportPreviewData, StudentBatchCreateResult, StudentTable} from "$types";
  import {invoke} from "@tauri-apps/api/core";
  import {open} from "@tauri-apps/plugin-dialog";
  import {overlayController} from "$controllers/overlayController";

  let previewData = $state<ImportPreviewData | null>(null);
  let filePath = $state("");
  let studentNoColumnIndex = $state(0);
  let nameColumnIndex = $state(1);
  let headerRows = $state(0);
  let isVisible = $state(false);
  let closeOnOutside = true;
  let isPreviewing = $state(false);
  let isImporting = $state(false);
  let message = $state<{ kind: "success" | "warn" | "info" | "error"; text: string } | null>(null);
  // 已删除但姓名不同的冲突记录，需用户逐条决策
  let pendingDecisions = $state<StudentTable[]>([]);
  // 学号 -> 是否覆盖（true=覆盖并恢复，false=跳过）
  let pendingChoices = $state<Record<string, boolean>>({});

  let configValid = $derived(
    previewData != null &&
    filePath !== "" &&
    studentNoColumnIndex >= 0 &&
    nameColumnIndex >= 0 &&
    studentNoColumnIndex !== nameColumnIndex &&
    headerRows >= 0 &&
    headerRows < (previewData?.total_rows ?? 0)
  );
  let allDecided = $derived(
    pendingDecisions.length > 0 &&
    pendingDecisions.every((s) => pendingChoices[s.student_no] !== undefined)
  );

  function resetState() {
    previewData = null;
    filePath = "";
    studentNoColumnIndex = 0;
    nameColumnIndex = 1;
    headerRows = 0;
    message = null;
    pendingDecisions = [];
    pendingChoices = {};
    isPreviewing = false;
    isImporting = false;
  }

  function openDialog() {
    resetState();
    isVisible = true;
  }

  function close() {
    isVisible = false;
    resetState();
  }

  async function chooseFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
      });
      if (!selected) return;
      filePath = selected;
      await preview();
    } catch (e) {
      message = { kind: "error", text: "选择文件失败：" + e };
    }
  }

  async function preview() {
    if (!filePath) return;
    isPreviewing = true;
    message = null;
    pendingDecisions = [];
    pendingChoices = {};
    try {
      const result = await invoke<ImportPreviewData>("preview_excel", {
        filePath: filePath,
      });
      previewData = result;
      // 默认：第 1 列学号、第 2 列姓名（列数不足则留空待配）
      studentNoColumnIndex = result.total_columns > 0 ? 0 : -1;
      nameColumnIndex = result.total_columns > 1 ? 1 : -1;
      headerRows = 0;
    } catch (e) {
      previewData = null;
      message = { kind: "error", text: "预览失败：" + e };
    } finally {
      isPreviewing = false;
    }
  }

  /** 列映射变化后，之前的冲突决策不再适用，需要重新导入校验 */
  function onConfigChange() {
    pendingDecisions = [];
    pendingChoices = {};
  }

  /** 点击表头按钮：设置 / 取消某列为学号列或姓名列 */
  function assignColumn(index: number, role: "student_no" | "name") {
    if (role === "student_no") {
      if (studentNoColumnIndex === index) {
        studentNoColumnIndex = -1;
        return;
      }
      studentNoColumnIndex = index;
      if (nameColumnIndex === index) nameColumnIndex = -1;
    } else {
      if (nameColumnIndex === index) {
        nameColumnIndex = -1;
        return;
      }
      nameColumnIndex = index;
      if (studentNoColumnIndex === index) studentNoColumnIndex = -1;
    }
    onConfigChange();
  }

  function chooseDecision(studentNo: string, override: boolean) {
    const next = {...pendingChoices};
    if (next[studentNo] === override) {
      delete next[studentNo];
    } else {
      next[studentNo] = override;
    }
    pendingChoices = next;
  }

  function decideAll(override: boolean) {
    const next: Record<string, boolean> = {...pendingChoices};
    for (const s of pendingDecisions) {
      next[s.student_no] = override;
    }
    pendingChoices = next;
  }

  async function runImport() {
    if (!configValid || isImporting) return;
    // 存在待决策冲突时必须全部处理完才能导入
    if (pendingDecisions.length > 0 && !allDecided) return;

    const decisions: Record<string, boolean> = {...pendingChoices};
    isImporting = true;
    message = null;
    try {
      const result = await invoke<StudentBatchCreateResult>("import_excel", {
        filePath: filePath,
        header_rows: headerRows,
        column_mapping: {
          student_no: studentNoColumnIndex,
          name: nameColumnIndex,
        },
        decisions: decisions,
      });
      switch (result.type) {
        case "Insert":
        case "Upsert": {
          await studentStore.load();
          message = {
            kind: "success",
            text: result.type === "Upsert"
              ? `成功导入 ${result.data.length} 名学生（含自动恢复/覆写）`
              : `成功导入 ${result.data.length} 名学生`,
          };
          setTimeout(close, 1000);
          break;
        }
        case "DuplicateInput":
          message = {
            kind: "error",
            text: `导入数据中存在重复学号：${result.data.join("、")}，请去重后重试。`,
          };
          break;
        case "Conflict":
          message = {
            kind: "error",
            text: `以下学号已存在活跃记录，无法导入：${result.data
              .map((s) => `${s.student_no}（${s.name}）`)
              .join("、")}。`,
          };
          break;
        case "DecisionRequired":
          pendingDecisions = result.data;
          pendingChoices = {};
          message = {
            kind: "info",
            text: "以下学号存在已删除但姓名不同的记录，请逐条选择「覆盖并恢复」或「跳过」。",
          };
          break;
      }
    } catch (e) {
      message = { kind: "error", text: "导入失败：" + e };
    } finally {
      isImporting = false;
    }
  }

  $effect(() => {
    overlayController.register("StudentImport", {
      open: openDialog,
      close: close,
      isVisible: () => isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? close : undefined}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>导入学生</h3>

      <!-- 文件选择 -->
      <div class="import-file-row">
        <input
          type="text"
          readonly
          placeholder="请选择 Excel 文件（.xlsx / .xls）"
          value={filePath}
        />
        <button type="button" class="btn-secondary" onclick={chooseFile} disabled={isPreviewing}>
          {filePath ? "重新选择" : "选择文件"}
        </button>
      </div>

      <!-- 预览表格：点击表头中的按钮设置学号 / 姓名列 -->
      {#if isPreviewing}
        <p class="preview-info">正在读取文件...</p>
      {:else if previewData}
        <div class="preview-table-wrap">
          <table class="preview-table">
            <thead>
            <tr>
              <th class="col-rowno">#</th>
              {#each Array(previewData.total_columns) as _, colIndex}
                <th>
                  <div class="col-head">
                    <span class="col-label">第 {colIndex + 1} 列</span>
                    <span class="col-tag col-tag-no" class:active={studentNoColumnIndex == colIndex}>学号</span>
                    <span class="col-tag col-tag-name" class:active={nameColumnIndex == colIndex}>姓名</span>
                    <div class="col-actions">
                      <button
                        type="button"
                        class:active={studentNoColumnIndex == colIndex}
                        onclick={() => assignColumn(colIndex, "student_no")}
                      >设为学号</button>
                      <button
                        type="button"
                        class:active={nameColumnIndex == colIndex}
                        onclick={() => assignColumn(colIndex, "name")}
                      >设为姓名</button>
                    </div>
                  </div>
                </th>
              {/each}
            </tr>
            </thead>
            <tbody>
            {#each previewData.rows as row, rowIndex}
              <tr class:is-header={rowIndex < headerRows}>
                <td class="col-rowno">{rowIndex + 1}</td>
                {#each row as cell, colIndex}
                  <td
                    class:is-no-col={studentNoColumnIndex == colIndex}
                    class:is-name-col={nameColumnIndex == colIndex}
                  >{cell || ""}</td>
                {/each}
              </tr>
            {/each}
            </tbody>
          </table>
        </div>
        <p class="preview-info">
          共 {previewData.total_rows} 行 × {previewData.total_columns} 列（显示前 {previewData.rows.length} 行，灰色行将被视为表头跳过）
        </p>
      {/if}

      <!-- 配置 -->
      {#if previewData}
        <div class="config-grid">
          <label>
            表头行数
            <input
              type="number"
              min="0"
              max={Math.max(previewData.total_rows - 1, 0)}
              bind:value={headerRows}
              oninput={onConfigChange}
            />
          </label>
          <label>
            学号列索引
            <input
              type="number"
              min="-1"
              max={Math.max(previewData.total_columns - 1, 0)}
              bind:value={studentNoColumnIndex}
              oninput={onConfigChange}
            />
          </label>
          <label>
            姓名列索引
            <input
              type="number"
              min="-1"
              max={Math.max(previewData.total_columns - 1, 0)}
              bind:value={nameColumnIndex}
              oninput={onConfigChange}
            />
          </label>
        </div>
      {/if}

      <!-- 冲突决策 -->
      {#if pendingDecisions.length > 0}
        <div class="decide-panel">
          <div class="decide-head">
            <strong>处理学号冲突（{pendingDecisions.length} 条）</strong>
            <div class="decide-batch">
              <button type="button" class="btn-secondary" onclick={() => decideAll(true)}>全部覆盖</button>
              <button type="button" class="btn-secondary" onclick={() => decideAll(false)}>全部跳过</button>
            </div>
          </div>
          <table class="decide-table">
            <thead>
            <tr>
              <th>学号</th>
              <th>原记录（已删除）</th>
              <th>处理方式</th>
            </tr>
            </thead>
            <tbody>
            {#each pendingDecisions as s (s.id)}
              <tr>
                <td>{s.student_no}</td>
                <td>{s.name}</td>
                <td class="decide-actions">
                  <button
                    type="button"
                    class="btn-secondary"
                    class:chosen={pendingChoices[s.student_no] === true}
                    onclick={() => chooseDecision(s.student_no, true)}
                  >覆盖并恢复</button>
                  <button
                    type="button"
                    class="btn-secondary"
                    class:chosen={pendingChoices[s.student_no] === false}
                    onclick={() => chooseDecision(s.student_no, false)}
                  >跳过</button>
                </td>
              </tr>
            {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <!-- 结果消息 -->
      {#if message}
        <div
          class="msg-box"
          class:msg-success={message.kind == "success"}
          class:msg-warn={message.kind == "warn"}
          class:msg-info={message.kind == "info"}
          class:msg-error={message.kind == "error"}
        >{message.text}</div>
      {/if}

      <div class="dialog-actions">
        <button type="button" class="btn-secondary" onclick={close} disabled={isImporting}>关闭</button>
        <button
          type="button"
          onclick={runImport}
          disabled={!configValid || isImporting || (pendingDecisions.length > 0 && !allDecided)}
        >
          {#if isImporting}
            导入中...
          {:else if pendingDecisions.length > 0}
            导入（{pendingDecisions.filter((s) => pendingChoices[s.student_no] !== undefined).length}/{pendingDecisions.length} 已决策）
          {:else}
            导入
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .import-file-row {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .import-file-row input {
    flex: 1;
    min-width: 0;
    padding: 6px 10px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    font-size: 13px;
    background: #f8f9fa;
    color: #495057;
  }

  .preview-table-wrap {
    max-height: 220px;
    overflow: auto;
    border: 1px solid #e9ecef;
    border-radius: 6px;
    margin-bottom: 6px;
  }

  .preview-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .preview-table th,
  .preview-table td {
    padding: 4px 6px;
    border: 1px solid #e9ecef;
    white-space: nowrap;
  }

  .preview-table th {
    background: #f8f9fa;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .col-rowno {
    width: 32px;
    text-align: center;
    color: #adb5bd;
  }

  .preview-table tbody tr.is-header td {
    background: #f1f3f5;
    color: #868e96;
  }

  .preview-table tbody td.is-no-col {
    background: #e7f5ff;
  }

  .preview-table tbody td.is-name-col {
    background: #ebfbee;
  }

  .col-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: center;
  }

  .col-actions {
    display: flex;
    gap: 4px;
  }

  .col-actions button {
    padding: 1px 6px;
    font-size: 11px;
    border: 1px solid #ced4da;
    border-radius: 4px;
    background: #fff;
    cursor: pointer;
  }

  .col-actions button:hover {
    background: #f1f3f5;
  }

  .col-actions button.active {
    border-color: #4dabf7;
    background: #e7f5ff;
  }

  .col-tag {
    display: none;
    padding: 0 6px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
  }

  .col-tag.active {
    display: inline-block;
  }

  .col-tag-no {
    background: #4dabf7;
    color: #fff;
  }

  .col-tag-name {
    background: #40c057;
    color: #fff;
  }

  .preview-info {
    margin: 6px 0 12px 0;
    font-size: 12px;
    color: #868e96;
  }

  .config-grid {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
  }

  .config-grid label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: #495057;
  }

  .config-grid input {
    width: 80px;
    padding: 4px 8px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    font-size: 13px;
  }

  .decide-panel {
    margin-bottom: 12px;
    border: 1px solid #ffd43b;
    border-radius: 6px;
    padding: 10px;
    background: #fff9db;
  }

  .decide-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    font-size: 13px;
  }

  .decide-batch {
    display: flex;
    gap: 6px;
  }

  .decide-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .decide-table th,
  .decide-table td {
    padding: 4px 8px;
    border-bottom: 1px solid #ffe066;
    text-align: left;
  }

  .decide-actions {
    display: flex;
    gap: 6px;
  }

  .decide-actions button.chosen {
    border-color: #1971c2;
    background: #e7f5ff;
  }

  .msg-box {
    margin-bottom: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    font-size: 13px;
  }

  .msg-success {
    background: #ebfbee;
    color: #2b8a3e;
  }

  .msg-warn {
    background: #fff9db;
    color: #e67700;
  }

  .msg-info {
    background: #e7f5ff;
    color: #1971c2;
  }

  .msg-error {
    background: #fff5f5;
    color: #c92a2a;
  }
</style>
