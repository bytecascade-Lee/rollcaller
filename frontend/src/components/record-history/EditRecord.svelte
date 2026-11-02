<script lang="ts">
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import {RecordCommand} from "$commands";
  import {recordStore} from "$stores/recordStore.svelte";
  import type {RecordOverlayName, RollcallRecord} from "$types";
  import type {EntryRegistry} from "$routing/registry";
  import type {AppEntryPayload} from "$routing/app/entries.generated";
  import {AppEntry} from "$routing/app/entries.generated";
  import {clickOutside, findAnchor, POPUP_CENTERED, updatePosition} from "$actions";
  import {attendanceStatusStore} from "$stores/attendanceStatusStore.svelte";
  import {appNavigate} from "$routing/app/navigate";

  let {
    onselect,
    overlays,
  }: {
    /** 改完回报新的选择集（清空即 `[]`）。选中集归页面所有，弹窗不直接改它 */
    onselect: (ids: readonly bigint[]) => void;
    overlays: EntryRegistry<RecordOverlayName, AppEntryPayload>;
  } = $props();

  let isVisible = $state(false);
  /** 触发元素（查不到就居中）；同时兼作 clickOutside 的排除项 */
  let anchorEl = $state<HTMLElement | null>(null);

  /** 目标记录 id：打开那一刻从**载荷**快照下来；行按"记录仍然存在"从 store 派生 */
  let targetIds = $state<number[]>([]);
  /** 是否已用记录当前值预填过（只种一次，之后不读 store） */
  let seeded = $state(false);
  const targets = $derived(
    targetIds.map((id) => recordStore.get(id)).filter((record): record is RollcallRecord => record != null),
  );

  /** 请求关闭：地址回退到父级（关闭是导航，不是本地状态）；`close()` 只留给注册表执行真正的隐藏 */
  function requestClose() {
    appNavigate.leave(AppEntry.editRecord);
  }

  let updateStatus = $state(true);
  let updateRemark = $state(true);
  let attendanceStatus = $state<number | null>(null);
  let remark = $state("");
  let popoverStyle = $state("");

  /** 载荷为空（深链只有地址）⇒ 不开，降级为纯导航。形状直接取声明，不在此重写一遍 */
  function open(ids: AppEntryPayload["edit-record"] | undefined) {
    if (!ids || ids.length === 0) return;
    targetIds = ids;
    updateStatus = true;
    updateRemark = true;
    attendanceStatus = null;
    remark = "";
    seeded = false;
    anchorEl = findAnchor(AppEntry.editRecord);
    popoverStyle = anchorEl ? updatePosition(anchorEl) : POPUP_CENTERED;
    if (recordStore.records.length === 0) void recordStore.load();
    isVisible = true;
  }

  /**
   * 单条时用记录当前值预填；多选时不预填（各项取值可能不同）。
   *
   * `seeded` 一旦置位就不再读 store —— 否则列表刷新会把用户改到一半的值盖回去；
   * 同时这让"跨窗口投递时记录还没加载"也能在数据到达后补种。
   */
  $effect(() => {
    if (seeded || targetIds.length !== 1) return;
    const only = recordStore.get(targetIds[0]);
    if (only == null) return;
    attendanceStatus = only.attendance_status;
    remark = only.remark ?? "";
    seeded = true;
  });

  function close() {
    isVisible = false;
    targetIds = [];
  }

  async function update() {
    const wantStatus = updateStatus && attendanceStatus != null;
    const wantRemark = updateRemark && remark.trim();
    if (!wantStatus && !wantRemark) {
      alert("请选择状态或填写备注");
      return;
    }
    const ids = targets.map((record) => record.id);
    if (ids.length === 0) return;

    let records: RollcallRecord[];
    if (wantStatus && wantRemark) {
      records = await RecordCommand.update(ids, attendanceStatus as number, remark.trim());
    } else if (wantStatus) {
      records = await RecordCommand.update_attendance_status(ids, attendanceStatus as number);
    } else if (wantRemark) {
      records = await RecordCommand.update_remark(ids, remark.trim());
    } else {
      return;
    }
    records.forEach((record) => recordStore.upsert(record));
    // 改完的这批记录仍然存在，故"按存在性派生"不会让它们自己掉出选中集 —— 这里显式回报清空
    onselect([]);
    requestClose();
  }

  $effect(() => {
    overlays.register(AppEntry.editRecord, {
      open,
      close,
    });
    return () => overlays.unregister(AppEntry.editRecord);
  });
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="popup" style={popoverStyle} use:clickOutside={{callback: requestClose, exclude: anchorEl ?? undefined}}>
    {#if targets.length == 1}
      <h3 class="text-title">修改记录</h3>
    {:else}
      <h3 class="text-title">批量修改记录（共 {targets.length} 条）</h3>
    {/if}
    <div class="field">
      <span class="field-label">
        <input type="checkbox" bind:checked={updateStatus} />
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
                attendanceStatus = attendanceStatus === id ? null : id;
              }}
            >
              <AttendanceStatusBadge {id} selected={attendanceStatus == id} />
            </button>
          {/each}
        {/if}
      </span>
    </div>

    <div class="field">
      <span class="field-label">
        <input type="checkbox" bind:checked={updateRemark} />
        备注
      </span>
      <input type="text" disabled={!updateRemark} placeholder="批量添加备注" bind:value={remark} />
    </div>

    <div class="button-group">
      <button type="button" class="button" onclick={requestClose}> 取消 </button>
      <button
        type="button"
        class="button yes"
        disabled={(updateStatus && attendanceStatus == null) ||
          (updateRemark && remark.trim() == "") ||
          (!updateStatus && !updateRemark)}
        onclick={update}
      >
        确定
      </button>
    </div>
  </div>
{/if}
