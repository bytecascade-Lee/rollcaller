<script lang="ts">

  import {studentStore} from "$stores/studentStore.svelte"
  import {recordStore} from "$stores/recordStore.svelte"
  import {rollcallEngine} from "$services/RollcallEngine.svelte";
  import {RollcallPhase} from "$types/RollcallPhase";
  import {format} from "$utils/DataTimeUtils";
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import {RollcallRecord} from "$types/RollcallRecord";
  import {RecordGroupMetaData} from "$types/RecordGroupMetaData";

  const engine = rollcallEngine;
  let display = $derived<RollcallRecord[]>(
    recordStore.records.filter((r) => r.id > recordStore.boundaryPoint).reverse()
  );
  let groupInfo = $derived<RecordGroupMetaData[]>(group(display));

  $effect(() => {
    studentStore.load();
    recordStore.load();
  });
</script>

<div class="page">
  <div class="result-area">
    <div
      class="result-name"
      class:animating={engine.phase == RollcallPhase.Animating}
      class:has-result={engine.phase != RollcallPhase.Animating && engine.currentName !== "" && engine.currentName !== "等待点名"}
    >
      {engine.currentName || "等待点名"}
    </div>
  </div>

  <div class="control-bar">
    <div class="control-item">
      <label>点名次数</label>
      <input
        type="number"
        min="1"
        max={studentStore.students.length || 1}
        value={engine.totalTimes}
        oninput={(e) => engine.updateTotalTimes(Number(e.currentTarget.value))}
        disabled={engine.isRolling}
      />
    </div>

    <div class="control-item">
      <label>总人数</label>
      <span class="stat-value">{studentStore.students.length}</span>
    </div>

    <div class="control-item">
      <label>已完成</label>
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
              <td><AttendanceStatusBadge code={record.attendance_status}/></td>
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
