<script lang="ts">

  import {studentStore} from "$stores/studentStore.svelte"
  import {recordStore} from "$stores/recordStore.svelte"
  import {rollcallEngine} from "$services/RollcallEngine.svelte";
  import {RollcallPhase} from "$types";
  import {format} from "$utils/DataTimeUtils";
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import type {RecordGroupMetaData, RollcallRecord} from "$types";

  const engine = rollcallEngine;
  let display = $derived<RollcallRecord[]>(
    recordStore.records.filter((r) => r.id > recordStore.boundaryPoint).reverse()
  );
  let groupInfo = $derived<RecordGroupMetaData[]>(group(display));

  let phaseText = $derived.by(() => {
    switch (engine.phase) {
      case RollcallPhase.Idle:
        return engine.completedTimes > 0 ? "本轮已完成" : "等待开始";
      case RollcallPhase.Animating:
        return "名字滚动中 · 点击「停止点名」即可选定";
      case RollcallPhase.Picking:
        return "正在选人...";
      case RollcallPhase.Showing:
        return "已选定";
    }
  });

  $effect(() => {
    studentStore.load();
    recordStore.load();
  });
</script>

<div class="page rollcall-page">
  <!-- 上方 1/3：被选中的人 -->
  <section class="result-panel">
    <div class="result-phase">{phaseText}</div>
    <div
      class="result-name"
      class:animating={engine.phase == RollcallPhase.Animating}
      class:has-result={engine.phase != RollcallPhase.Animating && engine.currentName !== "" && engine.currentName !== "等待点名"}
    >
      {engine.currentName || "等待点名"}
    </div>
  </section>

  <!-- 中间：点名次数 / 总人数 / 完成次数 / 按钮 -->
  <section class="control-bar">
    <div class="control-item">
      <label>
        点名次数
        <input
          type="number"
          min="1"
          max={studentStore.students.length || 1}
          value={engine.totalTimes}
          oninput={(e) => engine.updateTotalTimes(Number(e.currentTarget.value))}
          disabled={engine.isRolling}
        />
      </label>
    </div>

    <div class="control-item">
      <span class="stat-label">总人数</span>
      <span class="stat-value">{studentStore.students.length}</span>
    </div>

    <div class="control-item">
      <span class="stat-label">已完成</span>
      <span class="stat-value">{engine.completedTimes}/{engine.totalTimes}</span>
    </div>

    <div class="control-item">
      <button
        class="btn toggle-btn"
        class:start={!engine.isRolling}
        class:stop={engine.isRolling}
        onclick={() => engine.toggle()}
      >
        {engine.isRolling ? "停止点名" : "开始点名"}
      </button>
    </div>
  </section>

  <!-- 下方 1/2：本轮点名记录 -->
  <section class="table-section">
    <h3 class="table-title">当前点名记录（{display.length}）</h3>
    {#if recordStore.isLoading}
      <div class="empty">数据加载中...</div>
    {:else if display.length == 0}
      <div class="empty">当前还未点名，请先点名</div>
    {:else}
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
              <td><AttendanceStatusBadge code={record.attendance_status}/></td>
              <td>{record.remark}</td>
              <td>{format(record.rollcall_at)}</td>
            </tr>
          {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>

<style>
  /* 结构布局：上 1/3 结果区 + 中间控制区 + 下 1/2 记录表 */
  .rollcall-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
  }

  /* 上方 1/3：结果展示区 */
  .result-panel {
    flex: 1 1 33%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 1px solid #e9ecef;
    border-radius: 8px;
    background: #fff;
  }

  .result-phase {
    font-size: 13px;
    color: #6c757d;
  }

  .result-name {
    font-size: 56px;
    font-weight: 700;
    color: #495057;
    line-height: 1.2;
    max-width: 90%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-name.animating {
    color: #e94f4f;
  }

  .result-name.has-result {
    color: #16a34a;
  }

  /* 中间：统计与按钮 */
  .control-bar {
    flex-shrink: 0;
    display: flex;
    align-items: flex-end;
    gap: 24px;
    padding: 12px 16px;
    border: 1px solid #e9ecef;
    border-radius: 8px;
    background: #fff;
  }

  .control-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .control-item label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .control-item label,
  .stat-label {
    font-size: 12px;
    color: #6c757d;
  }

  .control-item input {
    width: 80px;
    padding: 6px 8px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    font-size: 14px;
  }

  .stat-value {
    font-size: 20px;
    font-weight: 600;
    color: #2c3e50;
    line-height: 1.3;
  }

  .toggle-btn {
    padding: 8px 20px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
  }

  .toggle-btn.start {
    background: #16a34a;
    color: #fff;
  }

  .toggle-btn.stop {
    background: #dc2626;
    color: #fff;
  }

  /* 下方 1/2：表格区 */
  .table-section {
    flex: 1 1 50%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .table-title {
    margin: 0;
    font-size: 13px;
    color: #495057;
    flex-shrink: 0;
  }
</style>
