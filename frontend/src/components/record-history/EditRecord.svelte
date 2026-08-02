<script lang="ts">
  import {RecordCommand} from "$commands";
  import {recordStore} from "$stores/recordStore.svelte";
  import {STATUS_MAP, statusText, STATUS_COLORS, STATUS_DEFAULT_COLOR} from "$constants";
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
  <div class="popover-backdrop" onclick={close}></div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="edit-popover" style={popoverStyle} onclick={(e) => e.stopPropagation()}>
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
          {@const s = statusStyle(code)}
          <button
            class="status-btn"
            type="button"
            disabled={!updateStatus}
            style:--status-bg={s.bg}
            style:--status-color={s.color}
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

    <div class="button-group">
      <button
        type="button"
        class="btn"
        style:--btn-bg="var(--app-color-surface-muted)"
        style:--btn-color="var(--app-color-text)"
        onclick={close}
      >取消</button>
      <button type="button" class="btn" onclick={update}>确定</button>
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

  .edit-popover {
    display: flex;
    flex-direction: column;
    gap: var(--app-space-sm);
    padding: var(--app-space-md);
    border: var(--border-size-1) solid var(--app-color-border);
    border-radius: var(--app-radius-md);
    background: var(--app-color-surface);
    box-shadow: var(--app-shadow-lg);
    z-index: var(--app-layer-popover);
  }

  .edit-popover h3 {
    margin: 0;
    font-size: var(--app-font-size-md);
    color: var(--app-color-text);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: var(--app-size-control);
    padding: var(--app-space-xs) var(--app-space-md);
    border: none;
    border-radius: var(--app-radius-sm);
    background: var(--btn-bg, var(--app-color-primary));
    color: var(--btn-color, var(--sand-0));
    font-family: inherit;
    font-size: var(--app-font-size-sm);
    font-weight: var(--app-font-weight-medium);
    cursor: pointer;
    transition: filter 150ms var(--app-ease), opacity 150ms var(--app-ease);
  }

  .btn:hover {
    filter: brightness(.94);
  }

  .btn:disabled {
    opacity: var(--app-opacity-disabled);
    cursor: not-allowed;
  }

  .status-group {
    display: flex;
    flex-wrap: wrap;
    gap: var(--app-space-xs);
  }

  .status-btn {
    padding: var(--app-space-xxs) var(--app-space-sm);
    border: none;
    border-radius: var(--app-radius-round);
    font-size: var(--app-font-size-xs);
    cursor: pointer;
    background: var(--status-bg, var(--app-color-surface-muted));
    color: var(--status-color, var(--app-color-text-muted));
  }

  .status-btn.selected {
    outline: var(--border-size-2) solid var(--gray-9);
    outline-offset: var(--size-1);
  }
</style>
