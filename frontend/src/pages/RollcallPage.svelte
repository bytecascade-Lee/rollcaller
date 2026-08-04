<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte"
  import {recordStore} from "$stores/recordStore.svelte"
  import {rollcallEngine} from "$services/RollcallEngine.svelte";
  import type {RecordGroupMetaData, RollcallRecord} from "$types";
  import {RollcallPhase} from "$types";
  import {format} from "$utils/DataTimeUtils";
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";

  const engine = rollcallEngine;
  let {active = $bindable(false)} = $props();
  let display = $derived<RollcallRecord[]>(
    recordStore.records.filter((r) => r.id > recordStore.boundaryPoint).reverse()
  );
  let groupInfo = $derived<RecordGroupMetaData[]>(group(display));

  $effect(() => {
    studentStore.load();
    recordStore.load();
  });
</script>

<div class:active={active}>
  <section class="result">
    {#if display.length == 0}
      <span class="name waiting">等待点名</span>
    {:else}
      <span
        class="name"
        class:animating={engine.phase == RollcallPhase.Animating}
        class:displaying={engine.phase != RollcallPhase.Animating}
      >
        <!-- 当当前学生为null时，如果开始点名，上方区域会先变低，当展示名字时，才会变回来 -->
        <!-- 换为空白字符串，问题照旧 -->
        <!-- 当前先使用学生名单的第一个人临时修复一下 -->
        <!-- 后期将滚动时显示的名字抽取到当前页面 -->
        {engine.currentName || studentStore.students[0].name}
      </span>
    {/if}
  </section>

  <!-- 中间：点名次数 / 总人数 / 完成次数 / 按钮 -->
  <div class="toolbar control-bar">
    <label class="field">
      <span class="field-label">点名次数</span>
      <input
        type="number"
        min="1"
        max={studentStore.students.length || 1}
        style="height: 28px; width: 64px"
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
      class="control"
      style:background={engine.isRolling ? "var(--color-warn)" : "var(--color-success)"}
      onclick={() => engine.toggle()}
    >
      {engine.isRolling ? "停止点名" : "开始点名"}
    </button>
  </div>

  <section class="table-section">
    {#if recordStore.isLoading}
      <div class="state">数据加载中...</div>
    {:else if display.length == 0}
      <div class="state">当前还未点名，暂无记录，请先点名</div>
    {:else}
      <h3 class="title">
        当前点名记录（{display.length}）
      </h3>
      <div class="table">
        <table>
          <thead>
          <tr>
            <th
              style:border="0px"
              style:width="8px"
            ></th>
            <th style:display="none"></th>
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
                <td
                  rowspan={groupInfo[index].rowspan}
                  style:background-color={color}
                  style:border="0px"
                  style:width="8px"
                ></td>
              {/if}
              <td style:display="none"></td>
              <td>{index + 1}</td>
              <td>{record.name}</td>
              <td>{record.student_no}</td>
              <td>
                <AttendanceStatusBadge code={record.attendance_status}/>
              </td>
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
  .result {
    min-height: 100px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-lg);
    border-radius: var(--radius-md);
    background: var(--color-page);
  }

  .result .name {
    font-size: var(--font-size-fluid-3);
    font-weight: var(--font-weight-heavy);
    color: var(--color-text);
    line-height: var(--font-lineheight-1);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result .name.waiting {
    color: var(--color-disabled);
  }

  .result .name.animating {
    color: var(--color-warn);
  }

  .result .name.displaying {
    color: var(--color-info);
  }

  .control-bar {
    flex-shrink: 0;
    align-items: flex-end;
    padding: 0;
    background: var(--color-page);
    border: var(--border-size-xs) solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .control-bar .field {
    width: var(--size-11);
  }

  .stat-value {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    color: var(--color-text);
    line-height: var(--font-lineheight-1);
  }

  .control {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-xs) var(--space-md);
    border: none;
    border-radius: var(--radius-md);
    background: var(--color-success);
    color: var(--color-page);
    font-family: inherit;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: filter var(--transition-duration-md) var(--transition-ease), opacity var(--transition-duration-md) var(--transition-ease);
  }

  .control:hover {
    filter: brightness(.90);
  }

  .table-section {
    flex: 1 1 50%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
</style>
