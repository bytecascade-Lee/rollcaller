<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {studentManagementDialogController} from "$controllers/studentManagementDialogController";
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
    studentManagementDialogController.register("Delete", {
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
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>确认删除</h3>
      <p>确定删除选中的 {selected.size} 条记录吗？</p>
      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => isVisible = false}>取消</button>
        <button class="btn-danger" onclick={del}>删除</button>
      </div>
    </div>
  </div>
{/if}
