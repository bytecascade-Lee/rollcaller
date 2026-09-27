<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {EntryRegistry} from "$routing/registry";
  import type {StudentOverlayName} from "$pages/overlays";
  import {AppEntry} from "$routing/entries.generated";
  import {StudentCommand} from "$commands";

  let {selected, hiddenCount, overlays} = $props<{
    /** 生效的选择集（页面已按"学生仍然存在"过滤） */
    selected: Set<bigint>;
    /** 其中不在当前筛选结果里的条数，用于提醒"会删到看不见的行" */
    hiddenCount: number;
    overlays: EntryRegistry<StudentOverlayName>;
  }>();
  let isVisible = $state(false);
  let closeOnOutside = true;

  async function del() {
    try {
      const ids = Array.from<bigint>(selected);
      await StudentCommand.remove(ids);
      // 只移除数据：生效的选择集是按"学生仍然存在"派生的，故不必（也不该）手工清空
      studentStore.remove(ids);
      isVisible = false;
    } catch (e) {
      alert(String(e));
    }
  }

  $effect(() => {
    overlays.register(AppEntry.deleteStudent, {
      show: () => isVisible = true,
      hide: () => isVisible = false,
      isShown: () => isVisible
    })
    return () => overlays.unregister(AppEntry.deleteStudent);
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? () => isVisible = false : undefined}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-title error">确认删除</h3>
      <p class="text-content">确定删除选中的 {selected.size} 条记录吗？</p>
      {#if hiddenCount > 0}
        <p class="text-content">其中 {hiddenCount} 条不在当前筛选结果中。</p>
      {/if}
      <p class="text-content">注：当前版本暂无恢复入口，删除后不可直接恢复！</p>
      <div class="button-group">
        <button
          type="button"
          class="button"
          onclick={() => isVisible = false}
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
