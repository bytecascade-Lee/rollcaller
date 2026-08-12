<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {ImportPreviewData, StudentTable} from "$types";
  import {Result} from "$types"
  import {open} from "@tauri-apps/plugin-dialog";
  import {overlayController} from "$controllers/popupController";
  import {ImportCommand} from "$commands";
  import {
    ArrowFatRightIcon,
    ArrowLeftIcon,
    ArrowRightIcon,
    CheckCircleIcon,
    CheckIcon,
    FileXlsIcon,
    InfoIcon,
    UploadSimpleIcon,
    WarningCircleIcon,
    XCircleIcon,
    XIcon
  } from "phosphor-svelte";

  let previewData = $state<ImportPreviewData | null>(null);
  let filePath = $state("");
  let studentNoColumnIndex = $state(0);
  let nameColumnIndex = $state(1);
  let headerRows = $state(0);
  // 向导步骤：1 选择文件 → 2 配置列映射 → 3 导入与冲突处理
  let step = $state(1);
  let isVisible = $state(false);
  let isPreviewing = $state(false);
  let isImporting = $state(false);
  let importSucceeded = $state(false);
  let message = $state<{ type: Result; content: string } | null>(null);
  // 已删除但姓名不同的冲突记录，需用户逐条决策
  let pendingDecisions = $state<StudentTable[]>([]);
  // 学号 -> 是否覆盖（true=覆盖并恢复，false=跳过）
  let pendingChoices = $state<Map<string, boolean>>(new Map<string, boolean>());
  let autoCloseTimer: ReturnType<typeof setTimeout> | undefined;

  // 表头行数指针（拖拽左侧隐形列的箭头设置忽略行数）
  let wrapEl = $state<HTMLDivElement | undefined>(undefined);
  let dragging = $state(false);

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
  let fileName = $derived(filePath.split(/[\\/]/).pop() || filePath);

  function clearAutoClose() {
    if (autoCloseTimer !== undefined) {
      clearTimeout(autoCloseTimer);
      autoCloseTimer = undefined;
    }
  }

  function resetState() {
    clearAutoClose();
    previewData = null;
    filePath = "";
    studentNoColumnIndex = 0;
    nameColumnIndex = 1;
    headerRows = 0;
    step = 1;
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
    clearAutoClose();
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
      // 选择文件后自动预览并跳转到步骤二
      if (previewData) step = 2;
    } catch (e) {
      message = {type: Result.Error, content: "选择文件失败：" + e};
    }
  }

  /**
   * 预览文件。resetMapping=true 时按默认列序重置映射（选择新文件）；
   * 向导内重新校验时传 false，保留用户已配置的映射。
   */
  async function preview(resetMapping = true) {
    if (!filePath) return;
    isPreviewing = true;
    message = null;
    pendingDecisions = [];
    pendingChoices = new Map<string, boolean>();
    importSucceeded = false;
    try {
      const result = await ImportCommand.preview(filePath);
      previewData = result;
      if (resetMapping) {
        // 默认：第 1 列学号、第 2 列姓名（列数不足则留空待配）
        studentNoColumnIndex = result.total_columns > 0 ? 0 : -1;
        nameColumnIndex = result.total_columns > 1 ? 1 : -1;
        headerRows = 0;
      } else {
        // 越界索引回退到合法值，避免重新校验后列映射失效
        if (studentNoColumnIndex >= result.total_columns) {
          studentNoColumnIndex = result.total_columns > 0 ? 0 : -1;
        }
        if (nameColumnIndex >= result.total_columns) {
          nameColumnIndex = result.total_columns > 1 ? 1 : -1;
        }
        if (headerRows >= result.total_rows) {
          headerRows = Math.max(0, result.total_rows - 1);
        }
      }
    } catch (e) {
      previewData = null;
      message = {type: Result.Error, content: "预览失败：" + e};
    } finally {
      isPreviewing = false;
    }
  }

  /** 步骤一「下一步」：若尚未预览（如重新选择失败后）则先预览，再进入步骤二 */
  async function goStep2() {
    if (!filePath) return;
    if (previewData === null) {
      await preview();
    }
    if (previewData) step = 2;
  }

  /** 步骤二「导入」：进入步骤三并直接执行导入；若有审查结果（冲突）则先处理决策 */
  async function goImport() {
    if (!configValid || isImporting) return;
    step = 3;
    await runImport();
  }

  /** 列映射变化后，之前的冲突决策不再适用，需要重新导入校验 */
  function onConfigChange() {
    pendingDecisions = [];
    pendingChoices = new Map<string, boolean>();
  }

  /** 由指针 Y 坐标计算应忽略的行数：指向表头=0，指向数据行 i=忽略 0..i。位置统一相对内容顶部，兼容容器滚动 */
  function computeHeaderFromY(clientY: number): number {
    if (!wrapEl) return 0;
    const rows = Array.from(wrapEl.querySelectorAll("tbody tr")) as HTMLElement[];
    if (rows.length === 0) return 0;
    const wrapRect = wrapEl.getBoundingClientRect();
    const y = clientY - wrapRect.top + wrapEl.scrollTop;
    // 表头（列映射辅助行）区域：指向这里表示不忽略任何行
    const theadBottom = rows[0].getBoundingClientRect().top - wrapRect.top + wrapEl.scrollTop;
    if (y <= theadBottom) return 0;
    for (let i = 0; i < rows.length; i++) {
      const rect = rows[i].getBoundingClientRect();
      const top = rect.top - wrapRect.top + wrapEl.scrollTop;
      const bottom = rect.bottom - wrapRect.top + wrapEl.scrollTop;
      if (y >= top && y <= bottom) return i + 1;
    }
    return rows.length;
  }

  function moveArrow(clientY: number) {
    const h = computeHeaderFromY(clientY);
    if (h !== headerRows) {
      headerRows = h;
      onConfigChange(); // 表头行数变化 → 之前冲突决策作废
    }
  }

  function onArrowDown(e: PointerEvent) {
    if (isPreviewing || isImporting) return;
    e.preventDefault();
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    moveArrow(e.clientY);
  }

  function onArrowMove(e: PointerEvent) {
    if (!dragging) return;
    moveArrow(e.clientY);
  }

  function onArrowUp() {
    dragging = false;
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
            type: Result.Success,
            content: result.type === "Upsert"
              ? `成功导入 ${result.data.length} 名学生（含自动恢复/覆写）`
              : `成功导入 ${result.data.length} 名学生`,
          };
          // 导入成功后延时 3s 自动关闭，让用户看到成功消息
          clearAutoClose();
          autoCloseTimer = setTimeout(() => close(), 3000);
          break;
        }
        case "DuplicateInput":
          message = {
            type: Result.Error,
            content: `导入数据中存在重复学号：${result.data.join("、")}，请去重后重试。`,
          };
          break;
        case "Conflict":
          message = {
            type: Result.Warning,
            content: `以下学号已存在活跃记录，无法导入：${result.data
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
      message = {type: Result.Error, content: "导入失败：" + e};
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
  <div class="overlay">
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="popup"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="导入学生"
    >
      <h3 class="text-title">导入学生</h3>
      <span class="text-content">从 Excel 文件批量导入学生名单</span>

      <!-- 步骤引导 -->
      <ol class="stepper">
        <li class="step" class:done={step > 1} class:active={step === 1}>
          <span class="step-badge">{#if step > 1}<CheckIcon size={13}/>{:else}1{/if}</span>
          <span class="step-label">选择文件</span>
        </li>
        <li class="step-line" aria-hidden="true"></li>
        <li class="step" class:done={step > 2} class:active={step === 2}>
          <span class="step-badge">{#if step > 2}<CheckIcon size={13}/>{:else}2{/if}</span>
          <span class="step-label">配置表头行数及列映射</span>
        </li>
        <li class="step-line" aria-hidden="true"></li>
        <li class="step" class:done={importSucceeded} class:active={step === 3}>
          <span class="step-badge">{#if importSucceeded}<CheckIcon size={13}/>{:else}3{/if}</span>
          <span class="step-label">导入</span>
        </li>
      </ol>

      <div class="dialog-body">
        <!-- 步骤 1：选择文件 -->
        {#if step === 1}
          <section class="step-section">
            <div class="section-head">
              <span class="section-no">1</span>
              <span class="section-title">选择文件</span>
            </div>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div>
              <div
                class="card"
                onclick={chooseFile}
              >
                <FileXlsIcon size="36" class="file-picker-icon"/>
                <span class="text-content" title={filePath}>{filePath ? fileName : "选择Excel文件"}</span>
              </div>
            </div>
          </section>
        {:else if step === 2}
          <!-- 步骤 2：预览 + 列映射 -->
          <section class="step-section">
            <div class="section-head">
              <span class="section-no">2</span>
              <span class="section-title">预览与列映射</span>
              <span class="section-hint">在第一列拖动箭头忽略表头；在表头下拉选择「学号 / 姓名」列</span>
            </div>

            {#if isPreviewing}
              <div class="loading-box">
                <span class="spinner" aria-hidden="true"></span>
                <span>正在读取文件…</span>
              </div>
            {:else if previewData}
              <div class="step2-layout">
                <div class="table-area">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <!-- svelte-ignore a11y_interactive_supports_focus -->
                  <div class="preview-table-wrap" bind:this={wrapEl}>
                    <table class="preview-table">
                      <thead>
                      <tr>
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <!-- svelte-ignore a11y_interactive_supports_focus -->
                        <th
                          class="col-arrow"
                          role="slider"
                          aria-label="拖动设置要忽略的表头行数"
                          aria-valuemin={0}
                          aria-valuemax={previewData.rows.length}
                          aria-valuenow={headerRows}
                          onpointerdown={onArrowDown}
                          onpointermove={onArrowMove}
                          onpointerup={onArrowUp}
                          onpointercancel={onArrowUp}
                        >
                          {#if headerRows === 0}
                            <ArrowFatRightIcon size={20} weight="fill" style="color: var(--color-primary)"/>
                          {/if}
                        </th>
                        <th class="col-rowno">#</th>
                        {#each Array(previewData.total_columns) as _, colIndex}
                          <th
                            class:is-no-col={studentNoColumnIndex == colIndex}
                            class:is-name-col={nameColumnIndex == colIndex}
                          >
                            <div class="col-head">
                              <select
                                class="col-select"
                                class:sel-no={studentNoColumnIndex == colIndex}
                                class:sel-name={nameColumnIndex == colIndex}
                                value={studentNoColumnIndex == colIndex ? "student_no" : nameColumnIndex == colIndex ? "name" : ""}
                                onchange={(e) => onHeaderRoleChange(colIndex, e.currentTarget.value)}
                                aria-label={`第 ${colIndex + 1} 列的映射`}
                              >
                                <option value="">忽略此列</option>
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
                          <!-- svelte-ignore a11y_no_static_element_interactions -->
                          <!-- svelte-ignore a11y_interactive_supports_focus -->
                          <td
                            class="col-arrow"
                            role="slider"
                            aria-label="拖动设置要忽略的表头行数"
                            aria-valuemin={0}
                            aria-valuemax={previewData.rows.length}
                            aria-valuenow={headerRows}
                            onpointerdown={onArrowDown}
                            onpointermove={onArrowMove}
                            onpointerup={onArrowUp}
                            onpointercancel={onArrowUp}
                          >
                            {#if rowIndex === headerRows - 1}
                              <ArrowFatRightIcon size={20} weight="fill" style="color: var(--color-primary)"/>
                            {/if}
                          </td>
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
                    <span>共 {previewData.total_rows} 行 × {previewData.total_columns} 列 · 预览前 {previewData.rows.length}
                      行</span>
                  </div>
                </div>
              </div>
            {:else}
              <div class="state">
                <WarningCircleIcon size="18"/>
                <span class="text-subtitle error">预览失败，请重新选择文件</span>
              </div>
            {/if}
          </section>
        {:else}
          <!-- 步骤 3：导入与冲突处理 -->
          <section class="step-section">
            <div class="section-head">
              <span class="section-no">3</span>
              <span class="section-title">导入与冲突处理</span>
              {#if pendingDecisions.length > 0}
                <span class="section-hint">需处理 {pendingDecisions.length} 条冲突</span>
              {:else if isImporting}
                <span class="section-hint">正在导入…</span>
              {/if}
            </div>

            {#if pendingDecisions.length > 0}
              <div class="decide-panel">
                <div class="decide-head">
                  <div class="decide-title">
                    <WarningCircleIcon size="22"/>
                    <div>
                      <strong>发现 {pendingDecisions.length} 条学号冲突</strong>
                      <p>以下学号存在已删除但姓名不同的记录，需逐条决策后才能导入</p>
                    </div>
                  </div>
                  <div class="decide-all-group">
                    <button type="button" class="button error" onclick={() => decideAll(true)}>全部覆盖</button>
                    <button type="button" class="button warn" onclick={() => decideAll(false)}>全部跳过</button>
                  </div>
                </div>
                <div class="decide-progress">
                  <div class="progress-track">
                    <div class="progress-fill"
                         style={`width: ${pendingDecisions.length ? (decidedCount / pendingDecisions.length) * 100 : 0}%`}></div>
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
                            class="button error"
                            class:chosen={pendingChoices.get(s.student_no) === true}
                            onclick={() => chooseDecision(s.student_no, true)}
                          >
                            <CheckIcon size="15"/>
                            覆盖并恢复
                          </button>
                          <button
                            type="button"
                            class="button warn"
                            class:chosen={pendingChoices.get(s.student_no) === false}
                            onclick={() => chooseDecision(s.student_no, false)}
                          >
                            <XIcon size="15"/>
                            跳过
                          </button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                  </tbody>
                </table>
              </div>
            {:else if isImporting}
              <div class="loading-box">
                <span class="spinner" aria-hidden="true"></span>
                <span>正在导入…</span>
              </div>
            {/if}
          </section>
        {/if}

        <!-- 结果消息：所有步骤可见，避免预览/导入报错时用户看不到提示 -->
        {#if message}
          <div
            class="alert"
            class:alert-success={message.type == Result.Success}
            class:alert-error={message.type == Result.Error}
            class:alert-warning={message.type == Result.Warning}
            class:alert-info={message.type == Result.Info}
            role="status"
          >
            {#if message.type == Result.Success}
              <CheckCircleIcon size="18"/>
            {:else if message.type == Result.Error}
              <XCircleIcon size="18"/>
            {:else if message.type == Result.Warning}
              <WarningCircleIcon size="18"/>
            {:else}
              <InfoIcon size="18"/>
            {/if}
            <span>{message.content}</span>
          </div>
        {/if}
      </div>

      <div class="button-group">
        <button
          type="button"
          class="button"
          onclick={close}
          disabled={isImporting}
        >关闭
        </button>
        {#if step > 1}
          <button
            type="button"
            class="button"
            onclick={() => (step = step === 3 ? 2 : 1)}
            disabled={isImporting}
          >
            <ArrowLeftIcon size={16}/>
            上一步
          </button>
        {/if}
        {#if step === 1}
          <button
            type="button"
            class="button yes"
            onclick={goStep2}
            disabled={!filePath || isPreviewing}
          >
            {#if isPreviewing}
              <span class="spinner spinner-sm" aria-hidden="true"></span>
            {/if}
            下一步
            <ArrowRightIcon size={16}/>
          </button>
        {:else if step === 2}
          <button
            type="button"
            class="button yes"
            onclick={goImport}
            disabled={!configValid || isImporting}
          >
            <UploadSimpleIcon size={18}/>
            导入学生
          </button>
        {:else}
          <button
            type="button"
            class="button yes"
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
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>

  /* 步骤引导 */
  .stepper {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    list-style: none;
    margin: 0;
    padding: var(--space-xxs) var(--space-lg) var(--space-xs);
    border-bottom: var(--border-size-xxs) solid var(--border-color-3);
  }

  .step {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .step-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--size-md);
    height: var(--size-md);
    border: var(--border-size-xxs) solid var(--border-color-3);
    border-radius: var(--radius-round);
    background: var(--color-page);
    color: var(--text-color-secondary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-bold);
    transition: background-color var(--transition-duration-fast) var(--transition-ease), color var(--transition-duration-fast) var(--transition-ease), border-color var(--transition-duration-fast) var(--transition-ease);
  }

  .step-label {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-color-secondary);
    transition: color var(--transition-duration-fast) var(--transition-ease);
  }

  .step.done .step-badge {
    border-color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 12%, transparent);
    color: var(--color-primary);
  }

  .step.done .step-label {
    color: var(--color-primary);
  }

  .step.active .step-badge {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: var(--color-page);
  }

  .step.active .step-label {
    color: var(--text-color-primary);
  }

  .step-line {
    flex: 1;
    min-width: var(--size-md);
    height: var(--border-size-xxs);
    background: var(--border-color-3);
  }

  .dialog-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    padding: var(--space-lg);
  }

  /* 区块标题 */
  .step-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .section-no {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--size-sm);
    height: var(--size-sm);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-primary) 12%, transparent);
    color: var(--color-primary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-bold);
  }

  .section-title {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
    color: var(--text-color-primary);
  }

  .section-hint {
    margin-left: auto;
    font-size: var(--font-size-xs);
    color: var(--text-color-secondary);
  }

  /* 加载态 */
  .loading-box {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    min-height: var(--size-xl);
    border: var(--border-size-xxs) solid var(--border-color-3);
    border-radius: var(--radius-sm);
    color: var(--text-color-secondary);
    font-size: var(--font-size-sm);
  }

  .spinner {
    display: inline-block;
    width: var(--size-md);
    height: var(--size-md);
    border: var(--border-size-sm) solid color-mix(in srgb, var(--color-primary) 25%, transparent);
    border-top-color: var(--color-primary);
    border-radius: var(--radius-round);
    animation: spin 0.8s linear infinite;
  }

  .spinner-sm {
    width: var(--size-sm);
    height: var(--size-sm);
    border-width: var(--border-size-xxs);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* 步骤二：左侧配置 + 右侧预览表格 */
  .step2-layout {
    display: flex;
    align-items: flex-start;
    gap: var(--space-lg);
  }

  /* 预览表格：sticky 表头 + 固定首列 */
  .table-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .preview-table-wrap {
    max-height: 22rem;
    overflow: auto;
    border: var(--border-size-xxs) solid var(--border-color-3);
    border-radius: var(--radius-sm);
    background: var(--color-page);
  }

  /* 表头行数指针所在列：冻结在左侧、背景不透明，滚动时仍可操控箭头 */
  .preview-table th.col-arrow,
  .preview-table td.col-arrow {
    position: sticky;
    left: 0;
    width: 2rem;
    min-width: 1.5rem;
    padding: 0;
    border: none;
    background: var(--color-page);
    text-align: center;
    vertical-align: middle;
    cursor: pointer;
    touch-action: none;
    z-index: var(--layer-2);
  }

  .preview-table thead th.col-arrow {
    background: color-mix(in srgb, var(--gray-12) 5%, var(--color-page));
    z-index: var(--layer-3);
  }

  .preview-table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: var(--font-size-xs);
  }

  .preview-table th,
  .preview-table td {
    padding: var(--space-xs) var(--space-sm);
    border-right: var(--border-size-xxs) solid var(--border-color-3);
    border-bottom: var(--border-size-xxs) solid var(--border-color-3);
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
    background: color-mix(in srgb, var(--gray-12) 5%, var(--color-page));
    color: var(--text-color-primary);
    font-weight: var(--font-weight-medium);
  }

  .preview-table th.col-rowno,
  .preview-table td.col-rowno {
    position: sticky;
    left: 1.5rem; /* 让出左侧表头行数指针列 */
    width: var(--size-lg);
    min-width: var(--size-lg);
    border-left: var(--border-size-xxs) solid var(--border-color-3); /* 与指针列分隔 */
    text-align: center;
    color: var(--text-color-secondary);
    background: var(--color-page);
    z-index: var(--layer-1);
  }

  .preview-table thead th.col-rowno {
    z-index: var(--layer-3);
    background: color-mix(in srgb, var(--gray-12) 5%, var(--color-page));
  }

  .preview-table tbody tr.is-header td {
    background: color-mix(in srgb, var(--gray-12) 5%, var(--color-page));
    color: var(--text-color-secondary);
  }

  .preview-table tbody td.is-no-col {
    background: color-mix(in srgb, var(--color-primary) 8%, var(--color-page));
  }

  .preview-table tbody td.is-name-col {
    background: color-mix(in srgb, var(--green-6) 10%, var(--color-page));
  }

  .preview-table thead th.is-no-col {
    background: color-mix(in srgb, var(--color-primary) 8%, color-mix(in srgb, var(--gray-12) 5%, var(--color-page)));
  }

  .preview-table thead th.is-name-col {
    background: color-mix(in srgb, var(--green-6) 10%, color-mix(in srgb, var(--gray-12) 5%, var(--color-page)));
  }

  /* 表头：列号 + 内容预览 + 单一角色下拉 */
  .col-head {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-xxs);
    min-width: var(--size-xl);
  }

  .col-select {
    width: 100%;
    padding: var(--space-xxs) var(--space-xs);
    font-size: var(--font-size-xs);
    cursor: pointer;
    transition: border-color var(--transition-duration-fast) var(--transition-ease), background-color var(--transition-duration-fast) var(--transition-ease), color var(--transition-duration-fast) var(--transition-ease);
  }

  .col-select.sel-no {
    border-color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 10%, var(--color-page));
    color: var(--color-primary);
    font-weight: var(--font-weight-medium);
  }

  .col-select.sel-name {
    border-color: var(--green-6);
    background: color-mix(in srgb, var(--green-6) 12%, var(--color-page));
    color: var(--green-6);
    font-weight: var(--font-weight-medium);
  }

  /* 预览摘要 + 图例 */
  .preview-summary {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-sm);
    flex-wrap: wrap;
    font-size: var(--font-size-xs);
    color: var(--text-color-secondary);
  }

  /* 冲突决策区：暖色面板，与预览表格明显区分 */
  .decide-panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding: var(--space-md);
    border: var(--border-size-xxs) solid color-mix(in srgb, var(--color-warn) 45%, var(--border-color-3));
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-warn) 7%, var(--color-page));
    box-shadow: var(--shadow-sm);
  }

  .decide-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
    flex-wrap: wrap;
  }

  .decide-title {
    display: flex;
    align-items: flex-start;
    gap: var(--space-xs);
    min-width: 0;
  }

  .decide-title strong {
    display: block;
    font-size: var(--font-size-sm);
    color: var(--text-color-primary);
  }

  .decide-title p {
    margin: var(--space-xxs) 0 0;
    font-size: var(--font-size-xs);
    color: var(--text-color-secondary);
  }

  .decide-all-group {
    display: flex;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .decide-progress {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .progress-track {
    flex: 1;
    height: var(--space-xs);
    border-radius: var(--radius-round);
    background: color-mix(in srgb, var(--gray-12) 8%, transparent);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    border-radius: var(--radius-round);
    background: var(--color-primary);
    transition: width var(--transition-duration-md) var(--transition-ease);
  }

  .progress-label {
    font-size: var(--font-size-xs);
    color: var(--text-color-secondary);
    white-space: nowrap;
  }

  .decide-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
  }

  .decide-table th,
  .decide-table td {
    padding: var(--space-xs) var(--space-sm);
    text-align: left;
    border-bottom: var(--border-size-xxs) solid var(--border-color-3);
  }

  .decide-table thead th {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-color-secondary);
  }

  .decide-table tbody tr:last-child td {
    border-bottom: none;
  }

  .decide-table tbody tr {
    transition: background-color 100ms var(--transition-ease);
  }

  .decide-table tbody tr:hover {
    background: color-mix(in srgb, var(--gray-12) 4%, transparent);
  }

  .decide-no {
    font-weight: var(--font-weight-medium);
  }

  .decide-actions {
    display: flex;
    gap: var(--space-xs);
  }

  /* 结果消息：按类型着色 */
  .alert {
    display: flex;
    align-items: flex-start;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    border: var(--border-size-xxs) solid transparent;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    text-align: left;
  }

  .alert-success {
    border-color: color-mix(in srgb, var(--color-success) 45%, transparent);
    background: color-mix(in srgb, var(--color-success) 8%, transparent);
    color: var(--text-color-primary);
  }

  .alert-error {
    border-color: color-mix(in srgb, var(--color-error) 45%, transparent);
    background: color-mix(in srgb, var(--color-error) 8%, transparent);
    color: var(--text-color-primary);
  }

  .alert-warning {
    border-color: color-mix(in srgb, var(--color-warn) 50%, transparent);
    background: color-mix(in srgb, var(--color-warn) 10%, transparent);
    color: var(--text-color-primary);
  }

  .alert-info {
    border-color: color-mix(in srgb, var(--color-info) 45%, transparent);
    background: color-mix(in srgb, var(--color-info) 8%, transparent);
    color: var(--text-color-primary);
  }
</style>
