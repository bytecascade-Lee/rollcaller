<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte"
  import {recordStore} from "$stores/recordStore.svelte"
  import {rollcallEngine} from "$services/RollcallEngine.svelte";
  import type {RecordGroupMetaData, RollcallRecord} from "$types";
  import {RollcallPhase} from "$types";
  import {format} from "$utils/DataTimeUtils";
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import {ArrowDownIcon, ArrowsDownUpIcon, ArrowUpIcon, MinusIcon, PlusIcon} from "phosphor-svelte";
  import Switch from "$components/common/Switch.svelte";
  import {attendanceStatusStore} from "$stores/attendanceStatusStore.svelte";
  import { ttsMode } from "$controllers/TtsController.svelte.js";

  const engine = rollcallEngine;
  let {active = $bindable(false)} = $props();
  let sortKey = $state("rollcall_at")
  let isAsc = $state(true)
  let tableEl = $state<HTMLDivElement | null>(null)
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
    attendanceStatusStore.load();
  });

  // 表格按点名时间升序排列，新点名记录追加在最后一行。
  // 记录条数变化（新增/首次加载）时，将滚动容器自动滚到底部以显示最新记录。
  $effect(() => {
    const count = display.length;
    if (tableEl && sortKey == "rollcall_at" && isAsc) {
      tableEl.scrollTop = tableEl.scrollHeight;
    }
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
    <div
      class="field"
      style:flex-direction="column"
    >
      <span class="field-label">点名次数</span>
      <div
        style:display="flex"
        style:gap="2px"
      >
        <button
          class="icon-button"
          onclick={() => engine.updateTotalTimes(engine.totalTimes - 1)}
          disabled={engine.isRolling || engine.totalTimes == 1}
        >
          <MinusIcon/>
        </button>
        <input
          type="text"
          inputmode="none"
          value={engine.totalTimes}
          disabled={engine.isRolling}
          oninput={(e) => {
            const raw = e.currentTarget.value;
            const cleaned = raw.replace(/[^0-9]/g, '');
            if (cleaned === '') {
              // 如果清空了，设为最小值
              e.currentTarget.value = '1';
              return;
            }
            let val = Number(cleaned);
            const max = studentStore.students.length || 1;
            if (val < 1) val = 1;
            if (val > max) val = max;
            e.currentTarget.value = String(val);
            engine.updateTotalTimes(val);
          }}
          onblur={(e) => {
            // 失焦时最终校验
            const val = Number(e.currentTarget.value);
            const max = studentStore.students.length || 1;
            let finalVal = val;
            if (isNaN(finalVal) || finalVal < 1) finalVal = 1;
            if (finalVal > max) finalVal = max;
            e.currentTarget.value = String(finalVal);
            engine.updateTotalTimes(finalVal);
          }}
          onkeydown={e => {
            if (!/^[0-9]$/.test(e.key) &&
              e.key !== 'Backspace' &&
              e.key !== 'Delete' &&
              e.key !== 'ArrowLeft' &&
              e.key !== 'ArrowRight' &&
              e.key !== 'Home' &&
              e.key !== 'End' &&
              e.key !== 'Tab'
              ) {
                e.preventDefault()
            }
          }}
        />
        <button
          class="icon-button"
          onclick={() => engine.updateTotalTimes(engine.totalTimes + 1)}
          disabled={engine.isRolling || engine.totalTimes == (studentStore.students.length || 1)}
        >
          <PlusIcon/>
        </button>
      </div>
    </div>

    <div class="field">
      <span class="field-label">总人数</span>
      <span class="stat-value">{studentStore.students.length}</span>
    </div>

    <div class="field">
      <span class="field-label">已完成</span>
      <span class="stat-value">{engine.completedTimes}/{engine.totalTimes}</span>
    </div>

    <div class="field">
      <span class="field-label">{engine.allowRepetition ? "允许重复" : "禁止重复"}</span>
      <button
        class="switch-button"
        onclick={() => (engine.allowRepetition = !engine.allowRepetition)}
        type="button"
      >
        <Switch yes={engine.allowRepetition}/>
      </button>
    </div>

    <div class="field">
      <span class="field-label">{ttsMode.value === "cloud" ? "云端语音" : "本地语音"}</span>
      <button
        class="switch-button"
        onclick={() => ttsMode.value = ttsMode.value === "local" ? "cloud" : "local"}
        type="button"
        disabled={engine.isRolling}
      >
        <Switch yes={ttsMode.value === "cloud"}/>
      </button>
    </div>

    <div class="field">
      {#if engine.isRolling}
        <button
          class="button warn rollcall-button"
          onclick={() => engine.toggle()}
        >
          停止点名
        </button>
      {:else}
        <button
          class="button yes rollcall-button"
          onclick={() => engine.toggle()}
        >
          开始点名
        </button>
      {/if}
    </div>
  </div>

  <section class="table-section">
    {#if recordStore.isLoading}
      <div class="state">数据加载中...</div>
    {:else if display.length == 0}
      <div class="state">当前暂无记录</div>
    {:else}
      <h3 class="text-title">
        当前点名记录（{display.length}）
      </h3>
      <div
        class="table"
        bind:this={tableEl}
      >
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
                  style:visibility={(sortKey == "rollcall_at") ? "visible" : "hidden"}
                ></td>
              {/if}
              <td style:display="none"></td>
              <td>{index + 1}</td>
              <td>{record.name}</td>
              <td>{record.student_no}</td>
              <td>
                <AttendanceStatusBadge id={record.attendance_status}/>
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
    min-height: 150px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-xxs);
    border-radius: var(--radius-md);
    background: var(--color-page);
  }

  .result .name {
    font-size: 90px;
    font-weight: var(--font-weight-heavy);
    color: var(--color-text);
    line-height: var(--font-lineheight-1);
    padding: 0;
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
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
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

  .field {
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .field .field-label {
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
  }

  .field input {
    flex: none;
    height: 28px;
    width: 48px;
    box-sizing: border-box;
  }

  .rollcall-button {
    width: 96px;
    /* 横跨标签行(22px) + 间距(--space-xs) + 内容行(28px)，与其它区域等高且更醒目 */
    height: calc(22px + var(--space-xs) + 28px);
    box-sizing: border-box;
    font-size: var(--font-size-lg);
    margin-left: 0;
  }

</style>
