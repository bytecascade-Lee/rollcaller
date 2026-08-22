<script lang="ts">
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import {RecordCommand} from "$commands";
  import {recordStore} from "$stores/recordStore.svelte";
  import type {RollcallRecord} from "$types";
  import {overlayController} from "$controllers/popupController";
  import {clickOutside, updatePosition} from "$actions";
  import {attendanceStatusStore} from "$stores/attendanceStatusStore.svelte";

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

  export function open() {
    updateStatus = true;
    updateRemark = true;
    attendanceStatus = null;
    remark = "";
    popoverStyle = updatePosition(anchor);
    isVisible = true;
    if (selected.size == 1) {
      // 必须拷贝，否则拿到的是引用，表格中的照样会变
      let value: bigint = selected.values().next().value;
      const original = recordStore.get(value ? value : -1n);
      if (original != null) {
        attendanceStatus = original.attendance_status;
        remark = original.remark ? original.remark : "";
      }
    }
  }

  export function close() {
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
  <div
    class="popup"
    style={popoverStyle}
    use:clickOutside={{ callback: close, exclude: anchor }}
  >
    {#if selected.size == 1}
      <h3 class="text-title">修改记录</h3>
    {:else}
      <h3 class="text-title">批量修改记录（共 {selected.size} 条）</h3>
    {/if}
    <div class="field">
      <span class="field-label">
        <input type="checkbox" bind:checked={updateStatus}/>
        状态
      </span>

      <span class="badge-group">
        {#if attendanceStatusStore.isLoading() || attendanceStatusStore.validStatusIds == null}
          <span class="state">加载中……</span>
        {:else}
          {#each attendanceStatusStore.validStatusIds as id (id)}
            <button
              class="badge-button"
              type="button"
              disabled={!updateStatus}
              style:padding="0"
              onclick={(e) => {
              e.stopPropagation();
              attendanceStatus = attendanceStatus === id ? null : id
            }}
            >
              <AttendanceStatusBadge id={id} selected={attendanceStatus == id}/>
            </button>
          {/each}
        {/if}
      </span>
    </div>


    <div class="field">
      <span class="field-label">
        <input type="checkbox" bind:checked={updateRemark}/>
        备注
      </span>
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
        class="button"
        onclick={close}>
        取消
      </button>
      <button
        type="button"
        class="button yes"
        disabled={(updateStatus && attendanceStatus == null) || (updateRemark && remark.trim() == "") || (!updateStatus && !updateRemark)}
        onclick={update}
      >
        确定
      </button>
    </div>
  </div>
{/if}
