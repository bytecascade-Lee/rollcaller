<script lang="ts">
  import {RecordCommand} from "$commands";
  import {recordStore} from "$stores/recordStore.svelte";
  import {STATUS_MAP, statusText} from "$constants";
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
      isVisible: isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="popover-backdrop" onclick={close}></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="popover" style={popoverStyle} onclick={(e) => e.stopPropagation()}>
    <h3>批量修改出勤记录（共 {selected.size} 条）</h3>

    <div class="field">
      <span class="field-label">更新条目</span>
      <label><input type="checkbox" bind:checked={updateStatus}/> 状态</label>
      <label><input type="checkbox" bind:checked={updateRemark}/> 备注</label>
    </div>

    <div class="field">
      <span class="field-label">状态</span>
      <div class="status-group">
        {#each statusCodes as code (code)}
          <button
            class="status-btn"
            type="button"
            disabled={!updateStatus}
            data-status={code}
            class:selected={attendanceStatus == code}
            onclick={() => attendanceStatus = attendanceStatus == code ? null : code}>
            {statusText(code)}
          </button>
        {/each}
      </div>
    </div>

    <div class="field">
      <span class="field-label">备注</span>
      <input
        type="text"
        disabled={!updateRemark}
        placeholder="批量添加备注"
        bind:value={remark}
      />
    </div>

    <div class="popover-actions">
      <button type="button" class="btn-secondary" onclick={close}>取消</button>
      <button type="button" onclick={update}>确定</button>
    </div>
  </div>
{/if}

<style>
  .popover-backdrop {
    position: fixed;
    inset: 0;
    z-index: 998;
    background: transparent;
  }

  .popover {
    position: fixed;
    z-index: 999;
    background: #fff;
    border: 1px solid #dee2e6;
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    padding: 16px;
    box-sizing: border-box;
  }

  .popover h3 {
    margin: 0 0 12px 0;
    font-size: 14px;
  }

  .field {
    margin-bottom: 12px;
  }

  .field-label {
    display: block;
    font-size: 13px;
    color: #495057;
    margin-bottom: 6px;
  }

  .field label {
    font-size: 13px;
    margin-right: 12px;
  }

  .status-group {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .status-btn {
    padding: 3px 10px;
    border: 1px solid transparent;
    border-radius: 12px;
    font-size: 13px;
    cursor: pointer;
    background: var(--status-bg, #e5e7eb);
    color: var(--status-color, #374151);
  }

  .status-btn[data-status="0"] {
    --status-bg: #dc2626;
    --status-color: #fef2f2;
  }

  .status-btn[data-status="1"] {
    --status-bg: #16a34a;
    --status-color: #fef2f2;
  }

  .status-btn[data-status="2"] {
    --status-bg: #d97706;
    --status-color: #fffbeb;
  }

  .status-btn[data-status="3"] {
    --status-bg: #F1E710;
    --status-color: #232020;
  }

  .status-btn[data-status="4"] {
    --status-bg: #2563eb;
    --status-color: #eff6ff;
  }

  .status-btn.selected {
    outline: 2px solid #212529;
    outline-offset: 1px;
  }

  .field input[type="text"] {
    width: 100%;
    padding: 6px 10px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    font-size: 13px;
    box-sizing: border-box;
  }

  .popover-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
</style>
