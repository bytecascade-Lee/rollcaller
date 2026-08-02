<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {ImportPreviewData, StudentTable} from "$types";
  import {open} from "@tauri-apps/plugin-dialog";
  import {overlayController} from "$controllers/overlayController";
  import {ImportCommand} from "$commands/";
  import {
    CheckCircleIcon,
    CheckIcon,
    FileXlsIcon,
    InfoIcon,
    UploadSimpleIcon,
    WarningCircleIcon,
    XCircleIcon,
    XIcon
  } from "phosphor-svelte";

  type MessageType = "info" | "success" | "error" | "warning";

  let previewData = $state<ImportPreviewData | null>(null);
  let filePath = $state("");
  let studentNoColumnIndex = $state(0);
  let nameColumnIndex = $state(1);
  let headerRows = $state(0);
  let isVisible = $state(false);
  let closeOnOutside = true;
  let isPreviewing = $state(false);
  let isImporting = $state(false);
  let importSucceeded = $state(false);
  let message = $state<{ type: MessageType; text: string } | null>(null);
  // 已删除但姓名不同的冲突记录，需用户逐条决策
  let pendingDecisions = $state<StudentTable[]>([]);
  // 学号 -> 是否覆盖（true=覆盖并恢复，false=跳过）
  let pendingChoices = $state<Map<string, boolean>>(new Map<string, boolean>());

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
    pendingDecisions.every((s) => pendingChoices.get(s.student_no) !== undefined)
  );
  let decidedCount = $derived(
    pendingDecisions.filter((s) => pendingChoices.get(s.student_no) !== undefined).length
  );
  // 流程步骤：1 选择文件 → 2 配置列 → 3 导入
  let currentStep = $derived(filePath === "" ? 1 : previewData === null ? 2 : 3);
  let fileName = $derived(filePath.split(/[\\/]/).pop() || filePath);

  function resetState() {
    previewData = null;
    filePath = "";
    studentNoColumnIndex = 0;
    nameColumnIndex = 1;
    headerRows = 0;
    message = null;
    pendingDecisions = [];
    pendingChoices = new Map<string, boolean>();
    isPreviewing = false;
    isImporting = false;
    importSucceeded = false;
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
        filters: [{name: "Excel", extensions: ["xlsx", "xls"]}],
      });
      if (!selected) return;
      filePath = selected;
      await preview();
    } catch (e) {
      message = {type: "error", text: "选择文件失败：" + e};
    }
  }

  async function preview() {
    if (!filePath) return;
    isPreviewing = true;
    message = null;
    pendingDecisions = [];
    pendingChoices = new Map<string, boolean>();
    importSucceeded = false;
    try {
      const result = await ImportCommand.preview(filePath);
      previewData = result;
      // 默认：第 1 列学号、第 2 列姓名（列数不足则留空待配）
      studentNoColumnIndex = result.total_columns > 0 ? 0 : -1;
      nameColumnIndex = result.total_columns > 1 ? 1 : -1;
      headerRows = 0;
    } catch (e) {
      previewData = null;
      message = {type: "error", text: "预览失败：" + e};
    } finally {
      isPreviewing = false;
    }
  }

  /** 列映射变化后，之前的冲突决策不再适用，需要重新导入校验 */
  function onConfigChange() {
    pendingDecisions = [];
    pendingChoices = new Map<string, boolean>();
  }

  /** 表头下拉选择列角色（"" = 取消映射） */
  function onHeaderRoleChange(colIndex: number, role: string) {
    if (role === "student_no") {
      studentNoColumnIndex = colIndex;
      if (nameColumnIndex === colIndex) nameColumnIndex = -1;
    } else if (role === "name") {
      nameColumnIndex = colIndex;
      if (studentNoColumnIndex === colIndex) studentNoColumnIndex = -1;
    } else {
      if (studentNoColumnIndex === colIndex) studentNoColumnIndex = -1;
      if (nameColumnIndex === colIndex) nameColumnIndex = -1;
    }
    onConfigChange();
  }

  /** 某列前几个非空单元格值（跳过表头行），用于映射预览 */
  function columnPreview(colIndex: number): string[] {
    if (!previewData) return [];
    const values: string[] = [];
    for (let i = headerRows; i < previewData.rows.length; i++) {
      const cell = (previewData.rows[i][colIndex] ?? "").trim();
      if (cell !== "") {
        values.push(cell);
        if (values.length >= 3) break;
      }
    }
    return values;
  }

  function chooseDecision(studentNo: string, override: boolean) {
    const next = new Map(pendingChoices);
    if (next.get(studentNo) === override) {
      next.delete(studentNo);
    } else {
      next.set(studentNo, override);
    }
    pendingChoices = next;
  }

  function decideAll(override: boolean) {
    const next = new Map(pendingChoices);
    for (const s of pendingDecisions) {
      next.set(s.student_no, override)
    }
    pendingChoices = next;
  }

  async function runImport() {
    if (!configValid || isImporting) return;
    // 存在待决策冲突时必须全部处理完才能导入
    if (pendingDecisions.length > 0 && !allDecided) return;

    // Tauri IPC 按 JSON 序列化，Map 需转成普通对象才能正确传递
    const decisions = Object.fromEntries(pendingChoices);
    isImporting = true;
    message = null;
    try {
      const result = await ImportCommand.load(filePath, headerRows, {
          student_no: studentNoColumnIndex,
          name: nameColumnIndex,
        },
        decisions
      );
      switch (result.type) {
        case "Insert":
        case "Upsert": {
          await studentStore.load();
          importSucceeded = true;
          message = {
            type: "success",
            text: result.type === "Upsert"
              ? `成功导入 ${result.data.length} 名学生（含自动恢复/覆写）`
              : `成功导入 ${result.data.length} 名学生`,
          };
          break;
        }
        case "DuplicateInput":
          message = {
            type: "error",
            text: `导入数据中存在重复学号：${result.data.join("、")}，请去重后重试。`,
          };
          break;
        case "Conflict":
          message = {
            type: "warning",
            text: `以下学号已存在活跃记录，无法导入：${result.data
              .map((s: { student_no: string; name: string; }) => `${s.student_no}（${s.name}）`)
              .join("、")}。`,
          };
          break;
        case "DecisionRequired":
          pendingDecisions = result.data;
          pendingChoices = new Map<string, boolean>();
          // 冲突面板自带说明文案，无需额外弹消息
          break;
      }
    } catch (e) {
      message = {type: "error", text: "导入失败：" + e};
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
    <div
      class="dialog import-dialog"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="导入学生"
    >
      <!-- 标题 + 关闭 -->
      <div class="dialog-head">
        <div class="dialog-title-wrap">
          <h3>导入学生</h3>
          <p class="dialog-subtitle">从 Excel 文件批量导入学生名单</p>
        </div>
        <button
          type="button"
          class="icon-button dialog-close"
          onclick={close}
          disabled={isImporting}
          aria-label="关闭"
        >
          <XIcon size={16}/>
        </button>
      </div>

      <!-- 步骤引导 -->
      <ol class="stepper">
        <li class="step" class:done={filePath !== ""} class:active={currentStep === 1}>
          <span class="step-badge">{#if filePath !== ""}<CheckIcon size={13}/>{:else}1{/if}</span>
          <span class="step-label">选择文件</span>
        </li>
        <li class="step-line" aria-hidden="true"></li>
        <li class="step" class:done={previewData !== null} class:active={currentStep === 2}>
          <span class="step-badge">{#if previewData !== null}<CheckIcon size={13}/>{:else}2{/if}</span>
          <span class="step-label">配置列映射</span>
        </li>
        <li class="step-line" aria-hidden="true"></li>
        <li class="step" class:done={importSucceeded} class:active={currentStep === 3}>
          <span class="step-badge">{#if importSucceeded}<CheckIcon size={13}/>{:else}3{/if}</span>
          <span class="step-label">导入</span>
        </li>
      </ol>

      <div class="dialog-body">
        <!-- 步骤 1：选择文件 -->
        <section class="step-section">
          <div class="section-head">
            <span class="section-no">1</span>
            <span class="section-title">选择文件</span>
            <span class="section-hint">支持 .xlsx / .xls</span>
          </div>
          <div class="file-picker">
            <FileXlsIcon size={36} class="file-picker-icon"/>
            <div class="file-picker-meta">
              <div class="file-name" title={filePath}>{filePath ? fileName : "尚未选择文件"}</div>
              <div class="file-hint">导入前会自动预览文件前 5 行</div>
            </div>
            <button type="button" class="button file-picker-btn" onclick={chooseFile} disabled={isPreviewing || isImporting}>
              <UploadSimpleIcon size={16}/>
              {filePath ? "重新选择" : "选择文件"}
            </button>
          </div>
        </section>

        <!-- 步骤 2：预览 + 列映射 -->
        {#if filePath}
          <section class="step-section">
            <div class="section-head">
              <span class="section-no">2</span>
              <span class="section-title">预览与列映射</span>
              <span class="section-hint">在表头下拉选择「学号 / 姓名」列，或在下方微调列索引</span>
            </div>

            {#if isPreviewing}
              <div class="loading-box">
                <span class="spinner" aria-hidden="true"></span>
                <span>正在读取文件…</span>
              </div>
            {:else if previewData}
              <div class="preview-table-wrap">
                <table class="preview-table">
                  <thead>
                  <tr>
                    <th class="col-rowno">#</th>
                    {#each Array(previewData.total_columns) as _, colIndex}
                      <th
                        class:is-no-col={studentNoColumnIndex == colIndex}
                        class:is-name-col={nameColumnIndex == colIndex}
                      >
                        <div class="col-head">
                          <span class="col-label">
                            第 {colIndex + 1} 列
                            {#if columnPreview(colIndex)[0]}
                              <span class="col-sample">· {columnPreview(colIndex)[0]}</span>
                            {/if}
                          </span>
                          <select
                            class="col-select"
                            class:sel-no={studentNoColumnIndex == colIndex}
                            class:sel-name={nameColumnIndex == colIndex}
                            value={studentNoColumnIndex == colIndex ? "student_no" : nameColumnIndex == colIndex ? "name" : ""}
                            onchange={(e) => onHeaderRoleChange(colIndex, e.currentTarget.value)}
                            aria-label={`第 ${colIndex + 1} 列的映射`}
                          >
                            <option value="">无</option>
                            <option value="student_no">学号</option>
                            <option value="name">姓名</option>
                          </select>
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
              <div class="preview-summary">
                <span>共 {previewData.total_rows} 行 × {previewData.total_columns} 列 · 预览前 {previewData.rows.length} 行</span>
                <span class="legend">
                  <span class="legend-dot dot-no"></span>学号列
                  <span class="legend-dot dot-name"></span>姓名列
                  <span class="legend-dot dot-header"></span>表头行（将跳过）
                </span>
              </div>

              <div class="mapping-grid">
                <div class="mapping-card" class:unset={studentNoColumnIndex < 0}>
                  <div class="mapping-card-head">
                    <span class="role-badge role-no">学号列</span>
                    <span class="mapping-index">
                      {studentNoColumnIndex >= 0 ? `第 ${studentNoColumnIndex + 1} 列` : "未设置"}
                    </span>
                  </div>
                  <div class="mapping-preview">
                    {#if studentNoColumnIndex >= 0}
                      {#if columnPreview(studentNoColumnIndex).length > 0}
                        {#each columnPreview(studentNoColumnIndex) as v, i (i)}
                          <span class="preview-chip">{v}</span>
                        {/each}
                      {:else}
                        <span class="preview-empty">该列暂无数据</span>
                      {/if}
                    {:else}
                      <span class="preview-empty">未设置，请选择列</span>
                    {/if}
                  </div>
                  <label class="field mapping-field">
                    <span class="field-label">列索引（从 0 开始）</span>
                    <input
                      type="number"
                      min="-1"
                      max={Math.max(previewData.total_columns - 1, 0)}
                      bind:value={studentNoColumnIndex}
                      oninput={onConfigChange}
                    />
                  </label>
                </div>

                <div class="mapping-card" class:unset={nameColumnIndex < 0}>
                  <div class="mapping-card-head">
                    <span class="role-badge role-name">姓名列</span>
                    <span class="mapping-index">
                      {nameColumnIndex >= 0 ? `第 ${nameColumnIndex + 1} 列` : "未设置"}
                    </span>
                  </div>
                  <div class="mapping-preview">
                    {#if nameColumnIndex >= 0}
                      {#if columnPreview(nameColumnIndex).length > 0}
                        {#each columnPreview(nameColumnIndex) as v, i (i)}
                          <span class="preview-chip">{v}</span>
                        {/each}
                      {:else}
                        <span class="preview-empty">该列暂无数据</span>
                      {/if}
                    {:else}
                      <span class="preview-empty">未设置，请选择列</span>
                    {/if}
                  </div>
                  <label class="field mapping-field">
                    <span class="field-label">列索引（从 0 开始）</span>
                    <input
                      type="number"
                      min="-1"
                      max={Math.max(previewData.total_columns - 1, 0)}
                      bind:value={nameColumnIndex}
                      oninput={onConfigChange}
                    />
                  </label>
                </div>
              </div>

              <label class="field header-rows-field">
                <span class="field-label">表头行数</span>
                <input
                  type="number"
                  min="0"
                  max={Math.max(previewData.total_rows - 1, 0)}
                  bind:value={headerRows}
                  oninput={onConfigChange}
                />
                <span class="field-hint">前 N 行作为表头跳过（表格中显示为灰色行）</span>
              </label>
            {/if}
          </section>
        {/if}

        <!-- 冲突决策 -->
        {#if pendingDecisions.length > 0}
          <section class="decide-panel">
            <div class="decide-head">
              <div class="decide-title">
                <WarningCircleIcon size={22} class="decide-icon"/>
                <div>
                  <strong>发现 {pendingDecisions.length} 条学号冲突</strong>
                  <p>以下学号存在已删除但姓名不同的记录，需逐条决策后才能导入</p>
                </div>
              </div>
              <div class="decide-all-group">
                <button type="button" class="decide-all-btn override" onclick={() => decideAll(true)}>全部覆盖</button>
                <button type="button" class="decide-all-btn skip" onclick={() => decideAll(false)}>全部跳过</button>
              </div>
            </div>
            <div class="decide-progress">
              <div class="progress-track">
                <div class="progress-fill" style={`width: ${pendingDecisions.length ? (decidedCount / pendingDecisions.length) * 100 : 0}%`}></div>
              </div>
              <span class="progress-label">已决策 {decidedCount}/{pendingDecisions.length}</span>
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
                  <td><span class="decide-no">{s.student_no}</span></td>
                  <td>{s.name}</td>
                  <td>
                    <div class="decide-actions">
                      <button
                        type="button"
                        class="decide-btn override"
                        class:chosen={pendingChoices.get(s.student_no) === true}
                        onclick={() => chooseDecision(s.student_no, true)}
                      >
                        <CheckIcon size={15}/>
                        覆盖并恢复
                      </button>
                      <button
                        type="button"
                        class="decide-btn skip"
                        class:chosen={pendingChoices.get(s.student_no) === false}
                        onclick={() => chooseDecision(s.student_no, false)}
                      >
                        <XIcon size={15}/>
                        跳过
                      </button>
                    </div>
                  </td>
                </tr>
              {/each}
              </tbody>
            </table>
          </section>
        {/if}

        <!-- 结果消息 -->
        {#if message}
          <div class="alert alert-{message.type}" role="status">
            {#if message.type === "success"}
              <CheckCircleIcon size={18}/>
            {:else if message.type === "error"}
              <XCircleIcon size={18}/>
            {:else if message.type === "warning"}
              <WarningCircleIcon size={18}/>
            {:else}
              <InfoIcon size={18}/>
            {/if}
            <span>{message.text}</span>
          </div>
        {/if}
      </div>

      <!-- 底部操作 -->
      <div class="dialog-foot">
        <button
          type="button"
          class="button close-btn"
          onclick={close}
          disabled={isImporting}
        >关闭</button>
        <button
          type="button"
          class="button import-btn"
          onclick={runImport}
          disabled={!configValid || isImporting || (pendingDecisions.length > 0 && !allDecided)}
        >
          {#if isImporting}
            <span class="spinner spinner-sm" aria-hidden="true"></span>
            正在导入…
          {:else}
            <UploadSimpleIcon size={18}/>
            {#if pendingDecisions.length > 0}
              导入（{decidedCount}/{pendingDecisions.length} 已决策）
            {:else}
              导入学生
            {/if}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* 弹窗骨架：头部/主体/底部固定，主体内部滚动 */
  .import-dialog {
    width: min(1160px, 94vw);
    max-width: none;
    min-width: 0;
    max-height: 88vh;
    overflow: hidden;
    padding: 0;
    gap: 0;
  }

  .dialog-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--app-space-md);
    padding: var(--app-space-lg) var(--app-space-lg) var(--app-space-sm);
  }

  .dialog-subtitle {
    margin: var(--app-space-xxs) 0 0;
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  .dialog-close {
    flex-shrink: 0;
  }

  /* 步骤引导 */
  .stepper {
    display: flex;
    align-items: center;
    gap: var(--app-space-xs);
    list-style: none;
    margin: 0;
    padding: var(--app-space-sm) var(--app-space-lg) var(--app-space-md);
    border-bottom: var(--border-size-1) solid var(--app-color-border);
  }

  .step {
    display: flex;
    align-items: center;
    gap: var(--app-space-xs);
    flex-shrink: 0;
  }

  .step-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--size-5);
    height: var(--size-5);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-round);
    background: var(--app-color-page);
    color: var(--app-color-text-muted);
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-bold);
    transition: background-color 150ms var(--app-ease), color 150ms var(--app-ease), border-color 150ms var(--app-ease);
  }

  .step-label {
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
    color: var(--app-color-text-muted);
    transition: color 150ms var(--app-ease);
  }

  .step.done .step-badge {
    border-color: var(--app-color-primary);
    background: color-mix(in srgb, var(--app-color-primary) 12%, transparent);
    color: var(--app-color-primary);
  }

  .step.done .step-label {
    color: var(--app-color-primary);
  }

  .step.active .step-badge {
    border-color: var(--app-color-primary);
    background: var(--app-color-primary);
    color: var(--sand-0);
  }

  .step.active .step-label {
    color: var(--app-color-text);
  }

  .step-line {
    flex: 1;
    min-width: var(--size-6);
    height: var(--border-size-1);
    background: var(--app-color-border);
  }

  .dialog-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--app-space-lg);
    padding: var(--app-space-lg);
  }

  /* 区块标题 */
  .step-section {
    display: flex;
    flex-direction: column;
    gap: var(--app-space-sm);
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: var(--app-space-xs);
  }

  .section-no {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--size-4);
    height: var(--size-4);
    border-radius: var(--app-radius-sm);
    background: color-mix(in srgb, var(--app-color-primary) 12%, transparent);
    color: var(--app-color-primary);
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-bold);
  }

  .section-title {
    font-size: var(--app-font-size-sm);
    font-weight: var(--app-font-weight-bold);
    color: var(--app-color-text);
  }

  .section-hint {
    margin-left: auto;
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  /* 文件选择 */
  .file-picker {
    display: flex;
    align-items: center;
    gap: var(--app-space-md);
    padding: var(--app-space-md);
    border: var(--border-size-2) dashed var(--app-color-border);
    border-radius: var(--app-radius-md);
    background: var(--app-color-page);
    transition: border-color 150ms var(--app-ease);
  }

  .file-picker:hover {
    border-color: var(--app-color-primary);
  }

  .file-picker-icon {
    flex-shrink: 0;
    color: var(--app-color-primary);
  }

  .file-picker-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--app-space-xxs);
  }

  .file-name {
    font-size: var(--app-font-size-sm);
    font-weight: var(--app-font-weight-medium);
    color: var(--app-color-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-hint {
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  .file-picker-btn {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--app-space-xs);
  }

  /* 加载态 */
  .loading-box {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--app-space-sm);
    min-height: var(--size-13);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-sm);
    color: var(--app-color-text-muted);
    font-size: var(--app-font-size-sm);
  }

  .spinner {
    display: inline-block;
    width: var(--size-5);
    height: var(--size-5);
    border: var(--border-size-2) solid color-mix(in srgb, var(--app-color-primary) 25%, transparent);
    border-top-color: var(--app-color-primary);
    border-radius: var(--app-radius-round);
    animation: spin 0.8s linear infinite;
  }

  .spinner-sm {
    width: var(--size-4);
    height: var(--size-4);
    border-width: var(--border-size-1);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* 预览表格：sticky 表头 + 固定首列 */
  .preview-table-wrap {
    max-height: 22rem;
    overflow: auto;
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-sm);
    background: var(--app-color-page);
  }

  .preview-table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: var(--app-font-size-xs);
  }

  .preview-table th,
  .preview-table td {
    padding: var(--app-space-xs) var(--app-space-sm);
    border-right: var(--border-size-1) solid var(--app-color-border);
    border-bottom: var(--border-size-1) solid var(--app-color-border);
    white-space: nowrap;
    text-align: left;
  }

  .preview-table th:last-child,
  .preview-table td:last-child {
    border-right: none;
  }

  .preview-table tbody tr:last-child td {
    border-bottom: none;
  }

  .preview-table thead th {
    position: sticky;
    top: 0;
    z-index: var(--layer-2);
    background: color-mix(in srgb, var(--gray-12) 5%, var(--app-color-page));
    color: var(--app-color-text);
    font-weight: var(--app-font-weight-medium);
  }

  .preview-table th.col-rowno,
  .preview-table td.col-rowno {
    position: sticky;
    left: 0;
    width: var(--size-7);
    min-width: var(--size-7);
    text-align: center;
    color: var(--app-color-text-muted);
    background: var(--app-color-page);
    z-index: var(--layer-1);
  }

  .preview-table thead th.col-rowno {
    z-index: var(--layer-3);
    background: color-mix(in srgb, var(--gray-12) 5%, var(--app-color-page));
  }

  .preview-table tbody tr.is-header td {
    background: color-mix(in srgb, var(--gray-12) 5%, var(--app-color-page));
    color: var(--app-color-text-muted);
  }

  .preview-table tbody td.is-no-col {
    background: color-mix(in srgb, var(--app-color-primary) 8%, var(--app-color-page));
  }

  .preview-table tbody td.is-name-col {
    background: color-mix(in srgb, var(--green-6) 10%, var(--app-color-page));
  }

  .preview-table thead th.is-no-col {
    background: color-mix(in srgb, var(--app-color-primary) 8%, color-mix(in srgb, var(--gray-12) 5%, var(--app-color-page)));
  }

  .preview-table thead th.is-name-col {
    background: color-mix(in srgb, var(--green-6) 10%, color-mix(in srgb, var(--gray-12) 5%, var(--app-color-page)));
  }

  /* 表头：列号 + 内容预览 + 单一角色下拉 */
  .col-head {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--app-space-xxs);
    min-width: var(--size-11);
  }

  .col-label {
    color: var(--app-color-text-muted);
    font-size: var(--app-font-size-xs);
  }

  .col-sample {
    opacity: 0.75;
  }

  .col-select {
    width: 100%;
    padding: var(--app-space-xxs) var(--app-space-xs);
    font-size: var(--app-font-size-xs);
    cursor: pointer;
    transition: border-color 150ms var(--app-ease), background-color 150ms var(--app-ease), color 150ms var(--app-ease);
  }

  .col-select.sel-no {
    border-color: var(--app-color-primary);
    background: color-mix(in srgb, var(--app-color-primary) 10%, var(--app-color-page));
    color: var(--app-color-primary);
    font-weight: var(--app-font-weight-medium);
  }

  .col-select.sel-name {
    border-color: var(--green-6);
    background: color-mix(in srgb, var(--green-6) 12%, var(--app-color-page));
    color: var(--green-6);
    font-weight: var(--app-font-weight-medium);
  }

  /* 预览摘要 + 图例 */
  .preview-summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--app-space-sm);
    flex-wrap: wrap;
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  .legend {
    display: inline-flex;
    align-items: center;
    gap: var(--app-space-xxs);
  }

  .legend-dot {
    display: inline-block;
    width: var(--app-space-xs);
    height: var(--app-space-xs);
    margin-left: var(--app-space-xs);
    border-radius: var(--app-radius-round);
  }

  .legend-dot:first-child {
    margin-left: 0;
  }

  .dot-no {
    background: var(--app-color-primary);
  }

  .dot-name {
    background: var(--green-6);
  }

  .dot-header {
    background: color-mix(in srgb, var(--gray-12) 5%, var(--app-color-page));
    border: var(--border-size-1) solid var(--app-color-border);
  }

  /* 列映射配置卡片 */
  .mapping-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--app-space-md);
  }

  .mapping-card {
    display: flex;
    flex-direction: column;
    gap: var(--app-space-xs);
    padding: var(--app-space-sm) var(--app-space-md);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-md);
    background: var(--app-color-page);
    box-shadow: 0 1px 3px color-mix(in srgb, var(--gray-12) 8%, transparent);
    transition: border-color 150ms var(--app-ease), box-shadow 150ms var(--app-ease);
  }

  .mapping-card:focus-within {
    border-color: var(--app-color-primary);
  }

  .mapping-card.unset {
    border-style: dashed;
  }

  .mapping-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--app-space-sm);
  }

  .role-badge {
    padding: var(--app-space-xxs) var(--app-space-xs);
    border-radius: var(--app-radius-round);
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
  }

  .role-no {
    background: color-mix(in srgb, var(--app-color-primary) 12%, transparent);
    color: var(--app-color-primary);
  }

  .role-name {
    background: color-mix(in srgb, var(--green-6) 14%, transparent);
    color: var(--green-6);
  }

  .mapping-index {
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  .mapping-preview {
    display: flex;
    align-items: center;
    gap: var(--app-space-xxs);
    flex-wrap: wrap;
    min-height: var(--size-5);
  }

  .preview-chip {
    padding: var(--app-space-xxs) var(--app-space-xs);
    border-radius: var(--app-radius-sm);
    background: var(--app-color-hover);
    color: var(--app-color-text);
    font-size: var(--app-font-size-xs);
  }

  .preview-empty {
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  .mapping-field {
    width: 100%;
  }

  .header-rows-field {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--app-space-sm);
  }

  .header-rows-field .field-label {
    flex-shrink: 0;
  }

  .header-rows-field input {
    width: var(--size-11);
    flex-shrink: 0;
  }

  .field-hint {
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  /* 冲突决策区：暖色面板，与预览表格明显区分 */
  .decide-panel {
    display: flex;
    flex-direction: column;
    gap: var(--app-space-sm);
    padding: var(--app-space-md);
    border: var(--border-size-1) solid color-mix(in srgb, var(--app-color-warn) 45%, var(--app-color-border));
    border-radius: var(--app-radius-md);
    background: color-mix(in srgb, var(--app-color-warn) 7%, var(--app-color-page));
    box-shadow: 0 1px 3px color-mix(in srgb, var(--gray-12) 8%, transparent);
  }

  .decide-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--app-space-md);
    flex-wrap: wrap;
  }

  .decide-title {
    display: flex;
    align-items: flex-start;
    gap: var(--app-space-xs);
    min-width: 0;
  }

  .decide-icon {
    flex-shrink: 0;
    margin-top: var(--app-space-xxs);
    color: var(--app-color-warn);
  }

  .decide-title strong {
    display: block;
    font-size: var(--app-font-size-sm);
    color: var(--app-color-text);
  }

  .decide-title p {
    margin: var(--app-space-xxs) 0 0;
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
  }

  .decide-all-group {
    display: flex;
    gap: var(--app-space-xs);
    flex-shrink: 0;
  }

  .decide-all-btn {
    padding: var(--app-space-xs) var(--app-space-sm);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-sm);
    background: var(--app-color-page);
    font-family: inherit;
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
    cursor: pointer;
    transition: background-color 150ms var(--app-ease), border-color 150ms var(--app-ease), color 150ms var(--app-ease);
  }

  .decide-all-btn.override {
    border-color: var(--green-6);
    color: var(--green-6);
  }

  .decide-all-btn.override:hover {
    background: color-mix(in srgb, var(--green-6) 10%, transparent);
  }

  .decide-all-btn.skip:hover {
    background: var(--app-color-hover);
  }

  .decide-progress {
    display: flex;
    align-items: center;
    gap: var(--app-space-sm);
  }

  .progress-track {
    flex: 1;
    height: var(--app-space-xs);
    border-radius: var(--app-radius-round);
    background: color-mix(in srgb, var(--gray-12) 8%, transparent);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    border-radius: var(--app-radius-round);
    background: var(--app-color-primary);
    transition: width 200ms var(--app-ease);
  }

  .progress-label {
    font-size: var(--app-font-size-xs);
    color: var(--app-color-text-muted);
    white-space: nowrap;
  }

  .decide-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--app-font-size-sm);
  }

  .decide-table th,
  .decide-table td {
    padding: var(--app-space-xs) var(--app-space-sm);
    text-align: left;
    border-bottom: var(--border-size-1) solid var(--app-color-border);
  }

  .decide-table thead th {
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
    color: var(--app-color-text-muted);
  }

  .decide-table tbody tr:last-child td {
    border-bottom: none;
  }

  .decide-table tbody tr {
    transition: background-color 100ms var(--app-ease);
  }

  .decide-table tbody tr:hover {
    background: color-mix(in srgb, var(--gray-12) 4%, transparent);
  }

  .decide-no {
    font-weight: var(--app-font-weight-medium);
  }

  .decide-actions {
    display: flex;
    gap: var(--app-space-xs);
  }

  .decide-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--app-space-xxs);
    padding: var(--app-space-xxs) var(--app-space-sm);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-sm);
    background: var(--app-color-page);
    font-family: inherit;
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
    color: var(--app-color-text);
    cursor: pointer;
    transition: background-color 150ms var(--app-ease), border-color 150ms var(--app-ease), color 150ms var(--app-ease);
  }

  .decide-btn:hover {
    background: var(--app-color-hover);
  }

  .decide-btn.override.chosen {
    border-color: var(--green-6);
    background: var(--green-6);
    color: var(--green-0);
  }

  .decide-btn.skip.chosen {
    border-color: var(--app-color-border);
    background: color-mix(in srgb, var(--gray-12) 8%, transparent);
    color: var(--app-color-text-muted);
  }

  /* 结果消息：按类型着色 */
  .alert {
    display: flex;
    align-items: flex-start;
    gap: var(--app-space-xs);
    padding: var(--app-space-sm) var(--app-space-md);
    border: var(--border-size-1) solid transparent;
    border-radius: var(--app-radius-sm);
    font-size: var(--app-font-size-sm);
    text-align: left;
  }

  .alert svg {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .alert-success {
    background: color-mix(in srgb, var(--green-6) 10%, var(--app-color-page));
    border-color: color-mix(in srgb, var(--green-6) 45%, transparent);
    color: var(--green-6);
  }

  .alert-error {
    background: color-mix(in srgb, var(--red-6) 10%, var(--app-color-page));
    border-color: color-mix(in srgb, var(--red-6) 45%, transparent);
    color: var(--red-7);
  }

  .alert-warning {
    background: color-mix(in srgb, var(--yellow-6) 14%, var(--app-color-page));
    border-color: color-mix(in srgb, var(--yellow-6) 45%, transparent);
    color: color-mix(in srgb, var(--yellow-6) 40%, var(--gray-8));
  }

  .alert-info {
    background: color-mix(in srgb, var(--blue-5) 10%, var(--app-color-page));
    border-color: color-mix(in srgb, var(--blue-5) 45%, transparent);
    color: var(--blue-6);
  }

  /* 底部操作区 */
  .dialog-foot {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--app-space-sm);
    padding: var(--app-space-md) var(--app-space-lg);
    border-top: var(--border-size-1) solid var(--app-color-border);
    background: var(--app-color-page);
  }

  .close-btn {
    border: var(--border-size-1) solid var(--app-color-border);
    background: var(--app-color-page);
  }

  .import-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--app-space-xs);
    min-height: var(--app-size-control-lg);
    padding: var(--app-space-xs) var(--app-space-lg);
    background: var(--app-color-primary);
    color: var(--app-color-text-white);
    font-size: var(--app-font-size-sm);
  }

  .import-btn:hover {
    background: var(--app-color-primary);
    filter: brightness(0.93);
  }

  .import-btn:disabled {
    background: var(--app-color-primary);
    color: var(--app-color-text-white);
    opacity: var(--app-opacity-disabled);
    cursor: not-allowed;
  }
</style>
