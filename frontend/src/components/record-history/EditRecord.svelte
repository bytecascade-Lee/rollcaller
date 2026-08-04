<script lang="ts">
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import {RecordCommand} from "$commands";
  import {recordStore} from "$stores/recordStore.svelte";
  import {STATUS_COLORS, STATUS_DEFAULT_COLOR, STATUS_MAP} from "$constants";
  import type {RollcallRecord} from "$types";
  import {overlayController} from "$controllers/overlayController";

  let {selected = $bindable(), anchor = $bindable()} = $props<{
    selected: Set<bigint>;
    anchor: HTMLElement | null;
  }>();

  let isVisible = $state(false);
  let updateStatus = $state(true);
  let updateRemark = $state(true);
  let attendanceStatus = $state<number | null>(null);
  let remark = $state("");
  let popoverStyle = $state("");

  const statusCodes = Object.keys(STATUS_MAP).map(Number);

  function statusStyle(code: number) {
    return STATUS_COLORS[code] ?? STATUS_DEFAULT_COLOR;
  }

  function updatePosition() {
    const rect = anchor.getBoundingClientRect();
    popoverStyle = `position: fixed; top: ${rect.bottom + 6}px; left: ${rect.left}px; min-width: ${Math.max(rect.width, 280)}px;`;
  }

  export function open() {
    updateStatus = true;
    updateRemark = true;
    attendanceStatus = null;
    remark = "";
    updatePosition();
    isVisible = true;
  }

  export function close() {
    isVisible = false;
  }

  async function update() {
    const wantStatus = updateStatus && attendanceStatus != null;
    const wantRemark = updateRemark && remark.trim();
    if (!wantStatus && !wantRemark) {
      alert("请选择状态或填写备注");
      return;
    }
    const ids: bigint[] = Array.from(selected);
    let records: RollcallRecord[];
    if (wantStatus && wantRemark) {
      records = await RecordCommand.update(ids, attendanceStatus as number, remark.trim());
    } else if (wantStatus) {
      records = await RecordCommand.update_attendance_status(ids, attendanceStatus as number);
    } else if (wantRemark) {
      records = await RecordCommand.update_remark(ids, remark.trim());
    } else {
      alert("卧槽")
      records = []
    }
    records.forEach(record => recordStore.upsert(record));
    selected.clear();
    isVisible = false;
  }

  $effect(() => {
    overlayController.register("RecordEdit", {
      open: open,
      close: close,
      isVisible: () => isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div onclick={close}></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="popup"
    style={popoverStyle}
    onclick={(e) => e.stopPropagation()}
  >
    <h3>批量修改记录（共 {selected.size} 条）</h3>
    <div class="field-label">
      <label><input type="checkbox" bind:checked={updateStatus}/>状态</label>
      <div class="status-group">
        {#each statusCodes as code (code)}
          <button
            class="button"
            type="button"
            class:selected={attendanceStatus == code}
            disabled={!updateStatus}
            style:padding="var(--size-xxs)"
            onclick={() => attendanceStatus = attendanceStatus == code ? null : code}
          >
              <AttendanceStatusBadge code={code}/>
          </button>
        {/each}
      </div>
    </div>

    <div class="field-label">
      <label><input type="checkbox" bind:checked={updateRemark}/> 备注</label>
      <div class="field-label">
        <input
          type="text"
          disabled={!updateRemark}
          placeholder="批量添加备注"
          bind:value={remark}/>
      </div>
    </div>

    <div class="button-group">
      <button
        type="button"
        class="button"
        onclick={close}>
        取消
      </button>
      <button type="button"
              class="button yes"
              onclick={update}>
        确定
      </button>
    </div>
  </div>
{/if}
