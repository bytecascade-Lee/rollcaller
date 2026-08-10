<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte"
  import {recordStore} from "$stores/recordStore.svelte"
  import {rollcallEngine} from "$services/RollcallEngine.svelte";
  import type {RecordGroupMetaData, RollcallRecord} from "$types";
  import {RollcallPhase} from "$types";
  import {format} from "$utils/DataTimeUtils";
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import {ArrowDownIcon, ArrowsDownUpIcon, ArrowUpIcon} from "phosphor-svelte";

  const engine = rollcallEngine;
  let {active = $bindable(false)} = $props();
  let sortKey = $state("rollcall_at")
  let isAsc = $state(true)
  let display = $derived<RollcallRecord[]>([...recordStore.records]
    .filter((r) => r.id > recordStore.boundaryPoint)
    .sort((a, b) => {
      if (!sortKey) return 0;
      const key = sortKey as "name" | "student_no" | "attendance_status" | "remark" | "rollcall_at";
      const valA = a[key];
      const valB = b[key];
      let cmp: number;
      if (typeof valA === "string" && typeof valB === "string") {
        cmp = valA.localeCompare(valB, "zh-Hans-CN");
      } else if (typeof valA === "number" && typeof valB === "number") {
        cmp = valA - valB;
      } else {
        cmp = 0;
      }
      return isAsc ? cmp : -cmp;
    })
  );
  let groupInfo = $derived<RecordGroupMetaData[]>(group(display));

  function sort(key: string) {
    if (sortKey === key) {
      isAsc = !isAsc;
    } else {
      sortKey = key;
      isAsc = true;
    }
  }

  $effect(() => {
    studentStore.load();
    recordStore.load();
  });
</script>

<div class:active={active}>
  <section class="result">
    {#if display.length == 0 && engine.phase == RollcallPhase.Idle}
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

  <div class="toolbar">
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

    {#if engine.isRolling}
      <button
        class="button warn"
        onclick={() => engine.toggle()}
      >
        停止点名
      </button>
    {:else}
      <button
        class="button yes"
        onclick={() => engine.toggle()}
      >
        开始点名
      </button>
    {/if}
  </div>

  <section class="table-section">
    {#if recordStore.isLoading}
      <div class="state">数据加载中...</div>
    {:else if display.length == 0}
      <div class="state">当前还未点名，暂无记录，请先点名</div>
    {:else}
      <h3 class="text-title">
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
            <th style:cursor="auto">序号</th>
            <th onclick={() => sort("name")}>
              姓名
              {#if sortKey === "name"}
                {#if isAsc}
                  <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
                {:else}
                  <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
                {/if}
              {:else}
                <ArrowsDownUpIcon size="14"/>
              {/if}
            </th>
            <th onclick={() => sort("student_no")}>
              学号
              {#if sortKey === "student_no"}
                {#if isAsc}
                  <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
                {:else}
                  <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
                {/if}
              {:else}
                <ArrowsDownUpIcon size="14"/>
              {/if}
            </th>
            <th onclick={() => sort("attendance_status")}>
              状态
              {#if sortKey === "attendance_status"}
                {#if isAsc}
                  <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
                {:else}
                  <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
                {/if}
              {:else}
                <ArrowsDownUpIcon size="14"/>
              {/if}
            </th>
            <th onclick={() => sort("remark")}>
              备注
              {#if sortKey === "remark"}
                {#if isAsc}
                  <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
                {:else}
                  <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
                {/if}
              {:else}
                <ArrowsDownUpIcon size="14"/>
              {/if}
            </th>
            <th onclick={() => sort("rollcall_at")}>
              点名时间
              {#if sortKey === "rollcall_at"}
                {#if isAsc}
                  <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
                {:else}
                  <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
                {/if}
              {:else}
                <ArrowsDownUpIcon size="14"/>
              {/if}
            </th>
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
    user-select: none;
  }

  .result .name.waiting {
    color: var(--text-color-secondary);
  }

  .result .name.animating {
    color: var(--color-warn);
  }

  .result .name.displaying {
    color: var(--color-primary);
  }

  .stat-value {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    color: var(--color-text);
    line-height: var(--font-lineheight-1);
  }

  .table-section {
    flex: 1 1 50%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
</style>
