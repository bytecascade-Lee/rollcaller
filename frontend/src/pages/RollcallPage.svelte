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
  let {active = $bindable(false)} = $props();
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

<!-- 页面根节点由 .content > * 提供布局与激活态 -->
<div class:active={active}>
  <!-- 上方 1/3：被选中的人 -->
  <section class="state result-panel">
    <span class="result-phase">{phaseText}</span>
    <span
      class="result-name"
      class:animating={engine.phase == RollcallPhase.Animating}
      class:has-result={engine.phase != RollcallPhase.Animating && engine.currentName !== "" && engine.currentName !== "等待点名"}
    >
      {engine.currentName || "等待点名"}
    </span>
  </section>

  <!-- 中间：点名次数 / 总人数 / 完成次数 / 按钮 -->
  <div class="toolbar control-bar">
    <label class="field">
      <span class="field-label">点名次数</span>
      <input
        type="number"
        min="1"
        max={studentStore.students.length || 1}
        value={engine.totalTimes}
        oninput={(e) => engine.updateTotalTimes(Number(e.currentTarget.value))}
        disabled={engine.isRolling}
      />
    </label>

    <div class="field">
      <span class="field-label">总人数</span>
      <span class="stat-value">{studentStore.students.length}</span>
    </div>

    <div class="field">
      <span class="field-label">已完成</span>
      <span class="stat-value">{engine.completedTimes}/{engine.totalTimes}</span>
    </div>

    <button
      class="button toggle-btn"
      style:--button-bg={engine.isRolling ? "var(--app-color-danger)" : "var(--app-color-success)"}
      onclick={() => engine.toggle()}
    >
      {engine.isRolling ? "停止点名" : "开始点名"}
    </button>
  </div>

  <!-- 下方 1/2：本轮点名记录 -->
  <section class="table-section">
    <h3 class="table-title">当前点名记录（{display.length}）</h3>
    {#if recordStore.isLoading}
      <div class="state">数据加载中...</div>
    {:else if display.length == 0}
      <div class="state">当前还未点名，请先点名</div>
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
  .result-panel {
    flex: 1 1 33%;
    min-height: 0;
  }

  .result-phase {
    font-size: var(--app-font-size-sm);
    color: var(--app-color-text-soft);
  }

  .result-name {
    font-size: var(--font-size-fluid-3);
    font-weight: var(--app-font-weight-heavy);
    color: var(--app-color-text);
    line-height: var(--font-lineheight-1);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-name.animating {
    color: var(--app-color-danger);
  }

  .result-name.has-result {
    color: var(--app-color-success);
  }

  .control-bar {
    flex-shrink: 0;
    align-items: flex-end;
    padding: var(--app-space-sm) var(--app-space-md);
    background: var(--app-color-surface);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-sm);
  }

  .control-bar .field {
    width: var(--size-11);
  }

  .stat-value {
    font-size: var(--app-font-size-xl);
    font-weight: var(--app-font-weight-bold);
    color: var(--app-color-text);
    line-height: var(--font-lineheight-1);
  }

  .toggle-btn {
    min-width: var(--size-10);
  }

  .table-section {
    flex: 1 1 50%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--app-space-xs);
  }

  .table-section .state {
    flex: 1;
  }

  .table-title {
    margin: 0;
    font-size: var(--app-font-size-sm);
    color: var(--app-color-text-soft);
    flex-shrink: 0;
  }
</style>
