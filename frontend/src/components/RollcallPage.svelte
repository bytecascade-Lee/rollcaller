<script>

  import {invoke} from "@tauri-apps/api/core";

  /** @type {import('$lib/types').StudentTable[]} */
  let allStudents = $state([]);

  /** @type {import('$lib/types').RollcallRecord[]} */
  let allRecords = $state([]);

  let sessionId = $state("");
  let totalTimes = $state(1);
  let completedTimes = $state(0);

  // 点名阶段：idle | animating | showing
  let phase = $state("idle");
  let currentName = $state("等待点名");

  let animTimer = $state(null);

  /** 自动循环是否应当继续运行 */
  let autoGo = $state(false);

  let isRolling = $state(false);

  // ── 工具 ────────────────────────────────────────

  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  function startAnim() {
    if (animTimer) clearInterval(animTimer);
    animTimer = setInterval(() => {
      if (allStudents.length > 0) {
        const idx = Math.floor(Math.random() * allStudents.length);
        currentName = allStudents[idx].name;
      }
    }, 80);
  }

  function stopAnim() {
    if (animTimer) {
      clearInterval(animTimer);
      animTimer = null;
    }
  }

  const STATUS_MAP = {
    0: "缺勤",
    1: "出勤",
    2: "迟到",
    3: "早退",
    4: "请假",
  };

  function statusText(/** @type {number} */ code) {
    return STATUS_MAP[code] ?? `未知(${code})`;
  }

  function fmtTime(/** @type {number} */ ms) {
    if (ms == null) return "";
    const d = new Date(ms);
    const pad = (n) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }

  // ── 连续点名自动循环 ──────────────────────────

  async function runAutoCycle() {
    autoGo = true;
    const studentIds = allStudents.map((s) => s.id);

    while (autoGo && completedTimes < totalTimes) {
      // 阶段1：动画滚动
      phase = "animating";
      startAnim();
      await sleep(1000);
      if (!autoGo) break;

      // 阶段2：选人并写入 DB
      stopAnim();
      let record;
      try {
        record = await invoke("roll_call_pick", {
          studentIds,
          sessionId,
        });
      } catch (e) {
        stopAnim();
        autoGo = false;
        phase = "idle";
        currentName = "等待点名";
        alert("点名失败：" + e);
        return;
      }
      if (!autoGo) break;

      currentName = record.name;
      completedTimes++;
      allRecords = [...allRecords, record];
      phase = "showing";

      if (completedTimes >= totalTimes) break;

      // 阶段3：短暂展示
      await sleep(1000);
      if (!autoGo) break;
    }

    // 循环结束
    stopAnim();
    if (completedTimes >= totalTimes) {
      phase = "idle";
      isRolling = false;
      autoGo = false;
    }
    // autoGo == false 时由 toggle 处理
  }

  // ── Toggle 按钮 ─────────────────────────────

  async function toggle() {
    if (!isRolling) {
      // ── 开始点名 ──
      if (allStudents.length === 0) {
        alert("没有可点名的学生，请先添加学生");
        return;
      }
      sessionId = crypto.randomUUID().replace(/-/g, "").substring(0, 8);
      completedTimes = 0;
      currentName = "";
      isRolling = true;

      if (totalTimes === 1) {
        // 单次：启动动画，等待用户再次点击停止
        phase = "animating";
        startAnim();
      } else {
        // 连续：自动循环
        runAutoCycle();
      }
    } else {
      // ── 停止点名 ──
      if (totalTimes === 1) {
        // 单次：选人 → 写入 DB → 显示
        stopAnim();
        const studentIds = allStudents.map((s) => s.id);
        try {
          const record = await invoke("roll_call_pick", {
            studentIds,
            sessionId,
          });
          currentName = record.name;
          completedTimes = 1;
          allRecords = [...allRecords, record];
          isRolling = false;
          phase = "idle";
        } catch (e) {
          stopAnim();
          isRolling = false;
          phase = "idle";
          currentName = "等待点名";
          alert("点名失败：" + e);
        }
      } else {
        // 连续：中途停止
        stopAnim();
        autoGo = false;
        currentName = "等待点名";
        isRolling = false;
        phase = "idle";
      }
    }
  }

  // ── 初始加载 ────────────────────────────────────

  $effect(() => {
    invoke("list_all_students").then((students) => {
      allStudents = students;
    });
  });
</script>

<div class="page">
  <!-- ─── 结果显示区 ──────────────────────────── -->
  <div class="result-area">
    <div
      class="result-name"
      class:animating={phase === "animating"}
      class:has-result={phase !== "animating" && currentName !== "" && currentName !== "等待点名"}
    >
      {currentName || "等待点名"}
    </div>
  </div>
  <!-- ─── 控制区 ────────────────────────────── -->
  <div class="control-bar">
    <div class="control-item">
      <label>点名次数</label>
      <input
        type="number"
        min="1"
        max={allStudents.length || 1}
        bind:value={totalTimes}
        disabled={isRolling}
      />
    </div>

    <div class="control-item">
      <label>总人数</label>
      <span class="stat-value">{allStudents.length}</span>
    </div>

    <div class="control-item">
      <label>已完成</label>
      <span class="stat-value">{completedTimes}/{totalTimes}</span>
    </div>

    <div class="control-item">
      <button
        class="btn toggle-btn"
        class:start={!isRolling}
        class:stop={isRolling}
        onclick={toggle}
      >
        {isRolling ? "停止点名" : "开始点名"}
      </button>
    </div>
  </div>

  <!-- ─── 点名记录列表（当前会话累计） ────────── -->
  <div class="table-section">
    <h3>点名记录（{allRecords.length}）</h3>
    <div class="table-wrap">
      <table>
        <thead>
        <tr>
          <th>#</th>
          <th>姓名</th>
          <th>学号</th>
          <th>状态</th>
          <th>备注</th>
          <th>点名时间</th>
          <th>会话ID</th>
        </tr>
        </thead>
        <tbody>
        {#each allRecords as record, i}
          <tr>
            <td class="col-seq">{i + 1}</td>
            <td>{record.name}</td>
            <td class="col-no">{record.student_no}</td>
            <td>{statusText(record.attendance_status)}</td>
            <td class="col-remark">{record.remark ?? "—"}</td>
            <td class="col-time">{fmtTime(record.rollcall_at)}</td>
            <td class="col-sid">{record.session_id.slice(0, 8)}…</td>
          </tr>
        {/each}
        </tbody>
      </table>
      {#if allRecords.length === 0}
        <div class="empty">暂无记录</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    padding: 10px;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-sizing: border-box;
    overflow: hidden;
  }

  /* ── 结果展示区 ──────────────────────────── */
  .result-area {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 80px;
    background: #f8f9fa;
    border-radius: 8px;
    border: 1px solid #e9ecef;
    flex-shrink: 0;
  }

  .result-name {
    font-size: 36px;
    font-weight: 700;
    color: #aaa;
    transition: color 0.3s;
  }

  .result-name.animating {
    color: #f39c12;
  }

  .result-name.has-result {
    color: #27ae60;
  }

  /* ── 控制区 ────────────────────────────── */
  .control-bar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 14px;
    background: #fff;
    border-radius: 6px;
    border: 1px solid #e9ecef;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .control-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .control-item label {
    font-size: 12px;
    color: #6c757d;
  }

  .stat-value {
    font-size: 16px;
    font-weight: 600;
    color: #2c3e50;
  }

  input[type="number"] {
    width: 60px;
    padding: 4px 8px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    font-size: 13px;
    text-align: center;
  }

  input[type="number"]:disabled {
    background: #e9ecef;
    color: #6c757d;
  }

  /* ── 按钮 ────────────────────────────── */
  .toggle-btn {
    padding: 6px 18px;
    border: none;
    border-radius: 5px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
    user-select: none;
  }

  .toggle-btn.start {
    background: #3498db;
    color: #fff;
  }

  .toggle-btn.start:hover {
    background: #2980b9;
  }

  .toggle-btn.stop {
    background: #e74c3c;
    color: #fff;
  }

  .toggle-btn.stop:hover {
    background: #c0392b;
  }

  /* ── 表格区 ────────────────────────────── */
  .table-section {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .table-section h3 {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
    color: #2c3e50;
    flex-shrink: 0;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid #e9ecef;
    border-radius: 6px;
    position: relative;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  th {
    background: #f8f9fa;
    padding: 6px 8px;
    text-align: left;
    font-weight: 600;
    color: #495057;
    border-bottom: 2px solid #dee2e6;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  td {
    padding: 5px 8px;
    border-bottom: 1px solid #e9ecef;
    color: #2c3e50;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  tbody tr:hover {
    background: #f8f9fa;
  }

  .col-seq {
    width: 32px;
    text-align: center;
    color: #888;
  }

  .col-no {
    width: 90px;
  }

  .col-time {
    width: 140px;
    color: #666;
  }

  .col-sid {
    font-family: monospace;
    font-size: 11px;
    color: #6c757d;
  }

  .col-remark {
    max-width: 120px;
  }

  .empty {
    text-align: center;
    color: #aaa;
    padding: 32px;
  }
</style>
