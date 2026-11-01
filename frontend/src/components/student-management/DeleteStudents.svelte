<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {EntryRegistry} from "$routing/registry";
  import type {StudentOverlayName, StudentTable} from "$types";
  import type {AppEntryPayload} from "$routing/app.entries.generated";
  import {AppEntry} from "$routing/app.entries.generated";
  import {StudentCommand} from "$commands";
  import {appNavigate} from "$routing/appNavigate";

  let {overlays}: { overlays: EntryRegistry<StudentOverlayName, AppEntryPayload> } = $props();

  let isVisible = $state(false);
  let targets = $state<StudentTable[]>([])
  let showDetails = $state(false);


  function open(ids: number[] | undefined) {
    if (studentStore.students.length === 0) void studentStore.load();
    if (!ids || ids.length === 0) return;
    targets = ids.map(id => studentStore.get(id))
      .filter((student): student is StudentTable => student != null);
    showDetails = false;
    isVisible = true;
  }

  function close() {
    isVisible = false;
    showDetails = false;
  }

  function requestClose() {
    appNavigate.leave(AppEntry.deleteStudent);
  }

  async function doDelete() {
    if (targets.length === 0) return;
    try {
      const ids = targets.map((student) => student.id);
      await StudentCommand.remove(ids);
      studentStore.remove(ids);
      requestClose();
    } catch (e) {
      alert(String(e));
    }
  }

  $effect(() => {
    overlays.register(AppEntry.deleteStudent, {
      open,
      close,
    })
    return () => overlays.unregister(AppEntry.deleteStudent);
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={requestClose}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-title error">确认删除</h3>
      <p class="text-content">确定删除这 {targets.length} 名学生吗？</p>
      <button
        type="button"
        class="detail-toggle"
        onclick={() => showDetails = !showDetails}
      >
        {showDetails ? "收起详细信息" : "查看详细信息"}
      </button>
      {#if showDetails}
        <ul class="detail-list">
          {#each targets as student (student.id)}
            <li class="text-content">{student.name}（{student.student_no}）</li>
          {/each}
        </ul>
      {/if}
      <p class="text-content">当前版本暂无恢复入口，删除后不可直接恢复！</p>
      <div class="button-group">
        <button
          type="button"
          class="button"
          onclick={requestClose}
        >取消
        </button>
        <button
          type="button"
          class="button error"
          onclick={doDelete}
        >删除
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .detail-toggle {
    display: block;
    width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: var(--color-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .detail-list {
    max-height: 160px;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
  }
</style>
