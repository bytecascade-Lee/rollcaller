<script lang="ts">

  import {invoke} from "@tauri-apps/api/core";
  import {studentStore} from "$stores/studentStore.svelte"
  import {recordStore} from "$stores/recordStore.svelte"
  import {RollcallPhase} from "$types/RollcallPhase";
  import {format} from "$utils/DataTimeUtils";
  import {uuid} from "$utils/UuidUtils";
  import type {RollcallRecord} from "$types/RollcallRecord";
  import {statusText} from "$constants/AttendanceStatus";
  import {COLORS, group} from "$services/RecordService.svelte";
  import type {RecordGroupMetaData} from "$types/RecordGroupMetaData";

  let sessionId = $state("");
  let totalTimes = $state(1);
  let completedTimes = $state(0);

  let phase = $state(RollcallPhase.Idle);
  let currentName = $state("等待点名");

  let display = $derived<RollcallRecord[]>(recordStore.records.filter(record => record.id > recordStore.boundaryPoint).reverse())
  let groupInfo = $derived<RecordGroupMetaData[]>(group(display))

  let animTimer: number | null = $state(0);

  let autoGo = $state(false);

  let isRolling = $state(false);

  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  function startAnim() {
    if (animTimer) clearInterval(animTimer);
    animTimer = setInterval(() => {
      if (studentStore.students.length > 0) {
        const idx = Math.floor(Math.random() * studentStore.students.length);
        currentName = studentStore.students[idx].name;
      }
    }, 80);
  }

  function stopAnim() {
    if (animTimer) {
      clearInterval(animTimer);
      animTimer = null;
    }
  }

  async function runAutoCycle() {
    autoGo = true;
    const studentIds = studentStore.students.map((s) => s.id);

    while (autoGo && completedTimes < totalTimes) {
      // 阶段1：动画滚动
      phase = RollcallPhase.Animating;
      startAnim();
      await sleep(1000);
      if (!autoGo) break;

      // 阶段2：选人并写入 DB
      stopAnim();
      let record;
      try {
        record = await invoke<RollcallRecord>("roll_call_pick", {
          studentIds,
          sessionId,
        });
      } catch (e) {
        stopAnim();
        autoGo = false;
        phase = RollcallPhase.Idle;
        currentName = "等待点名";
        alert("点名失败：" + e);
        return;
      }
      if (!autoGo) break;

      currentName = record.name;
      completedTimes++;
      recordStore.upsert(record)
      phase = RollcallPhase.Showing;

      if (completedTimes >= totalTimes) break;

      // 阶段3：短暂展示
      await sleep(1000);
      if (!autoGo) break;
    }

    // 循环结束
    stopAnim();
    if (completedTimes >= totalTimes) {
      phase = RollcallPhase.Idle;
      isRolling = false;
      autoGo = false;
    }
    // autoGo == false 时由 toggle 处理
  }

  // ── Toggle 按钮 ─────────────────────────────

  async function toggle() {
    if (!isRolling) {
      // ── 开始点名 ──
      if (studentStore.students.length === 0) {
        alert("没有可点名的学生，请先添加学生");
        return;
      }
      sessionId = uuid();
      completedTimes = 0;
      currentName = "";
      isRolling = true;

      if (totalTimes === 1) {
        // 单次：启动动画，等待用户再次点击停止
        phase = RollcallPhase.Animating;
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
        const studentIds = studentStore.students.map((s) => s.id);
        try {
          const record = await invoke<RollcallRecord>("roll_call_pick", {
            studentIds,
            sessionId,
          });
          currentName = record.name;
          completedTimes = 1;
          recordStore.upsert(record)
          isRolling = false;
          phase = RollcallPhase.Idle;
        } catch (e) {
          stopAnim();
          isRolling = false;
          phase = RollcallPhase.Idle;
          currentName = "等待点名";
          alert("点名失败：" + e);
        }
      } else {
        // 连续：中途停止
        stopAnim();
        autoGo = false;
        currentName = "等待点名";
        isRolling = false;
        phase = RollcallPhase.Idle;
      }
    }
  }

  $effect(() => {
    studentStore.load();
    recordStore.load();
  });
</script>

<div class="page">
  <div class="result-area">
    <div
      class="result-name"
      class:animating={phase == RollcallPhase.Animating}
      class:has-result={phase != RollcallPhase.Animating && currentName !== "" && currentName !== "等待点名"}
    >
      {currentName || "等待点名"}
    </div>
  </div>

  <div class="control-bar">
    <div class="control-item">
      <label>点名次数</label>
      <input
        type="number"
        min="1"
        max={studentStore.students.length || 1}
        bind:value={totalTimes}
        disabled={isRolling}
      />
    </div>

    <div class="control-item">
      <label>总人数</label>
      <span class="stat-value">{studentStore.students.length}</span>
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

  {#if recordStore.isLoading}
    数据加载中...
  {:else if display.length == 0}
    当前还未点名，请先点名
  {:else }
    <div class="table-section">
      <h3>当前点名记录（{display.length}）</h3>
      <div class="table">
        <table>
          <thead>
          <tr>
            <th></th>
            <th>序号</th>
            <th>姓名</th>
            <th>学号</th>
            <th>状态</th>
            <th>备注</th>
            <th>点名时间</th>
          </tr>
          </thead>
          <tbody>
          {#each display as record, index (record.id)}
            {@const color = COLORS[groupInfo[index].groupIndex % COLORS.length]}
            <tr>
              {#if groupInfo[index].isStart}
                <td rowspan={groupInfo[index].rowspan} style:background-color={color}></td>
              {/if}
              <td>{index + 1}</td>
              <td>{record.name}</td>
              <td>{record.student_no}</td>
              <td>{statusText(record.attendance_status)}</td>
              <td>{record.remark}</td>
              <td>{format(record.rollcall_at)}</td>
            </tr>
          {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}

</div>
