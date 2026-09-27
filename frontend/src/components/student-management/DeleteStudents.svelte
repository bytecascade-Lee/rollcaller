<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {EntryRegistry} from "$routing/registry";
  import type {StudentOverlayName, StudentTable} from "$types";
  import {AppEntry} from "$routing/entries.generated";
  import {appLeave, readStudentIds} from "$routing/entries";
  import {StudentCommand} from "$commands";

  let {overlays}: {
    overlays: EntryRegistry<StudentOverlayName>;
  } = $props();

  let isVisible = $state(false);

  /** 请求关闭：地址回退到父级（关闭是导航，不是本地状态）；`close()` 只留给注册表执行真正的隐藏 */
  function requestClose() {
    appLeave(AppEntry.deleteStudent);
  }
  let closeOnOutside = true;
  let showDetails = $state(false);

  /**
   * 待删除的学生 id —— **打开那一刻从 payload 快照下来**，之后只认它。
   *
   * 于是弹窗不读页面状态（不再需要 `bind:selected`），调用方无论是工具栏按钮还是深链，
   * 都只负责给 payload。快照只存 id 不存行：展示用的行按"学生仍然存在"派生，
   * 因此"数据还没加载完"会自愈，不必命令式重取。
   */
  let targetIds = $state<bigint[]>([]);
  const targets = $derived(
    targetIds
      .map((id) => studentStore.get(id))
      .filter((student): student is StudentTable => student != null),
  );

  /** `{ids}` 非法或为空 ⇒ 什么都不做（降级为纯导航，不空页、不半应用） */
  function open(payload?: unknown) {
    const ids = readStudentIds(payload);
    if (ids.length === 0) return;
    targetIds = ids.map((id) => BigInt(id));
    showDetails = false;
    if (studentStore.students.length === 0) void studentStore.load();
    isVisible = true;
  }

  function close() {
    isVisible = false;
    showDetails = false;
  }

  async function del() {
    if (targets.length === 0) return;
    try {
      const ids = targets.map((student) => student.id);
      await StudentCommand.remove(ids);
      // 只移除数据：页面那边生效的选择集是按"学生仍然存在"派生的，故不必（也不该）手工清理
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
      isVisible: () => isVisible,
    })
    return () => overlays.unregister(AppEntry.deleteStudent);
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? requestClose : undefined}>
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
      <p class="text-content">注：当前版本暂无恢复入口，删除后不可直接恢复！</p>
      <div class="button-group">
        <button
          type="button"
          class="button"
          onclick={requestClose}
        >取消</button>
        <button
          type="button"
          class="button error"
          onclick={del}
        >删除</button>
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
