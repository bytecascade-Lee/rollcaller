<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {overlayController} from "$controllers/popupController";
  import {StudentCommand} from "$commands";

  let {selected = $bindable()} = $props<{ selected: Set<bigint> }>();
  let isVisible = $state(false);
  let closeOnOutside = true;

  async function del() {
    try {
      let ids = Array.from<bigint>(selected);
      await StudentCommand.remove(ids);
      studentStore.remove(ids);
      selected.clear();
      isVisible = false;
    } catch (e) {
      alert(String(e));
    }
  }

  $effect(() => {
    overlayController.register("StudentDelete", {
      open: () => isVisible = true,
      close: () => isVisible = false,
      isVisible: () => isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? () => isVisible = false : undefined}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-title error">确认删除</h3>
      <p class="text-content">确定删除选中的 {selected.size} 条记录吗？</p>
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
