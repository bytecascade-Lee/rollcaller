<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {overlayController} from "$controllers/overlayController";
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
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>确认删除</h3>
      <p>确定删除选中的 {selected.size} 条记录吗？</p>
      <div class="button-group">
        <button
          type="button"
          class="btn"
          style:--btn-bg="var(--app-color-surface-muted)"
          style:--btn-color="var(--app-color-text)"
          onclick={() => isVisible = false}
        >取消</button>
        <button
          type="button"
          class="btn"
          style:--btn-bg="var(--red-7)"
          onclick={del}
        >删除</button>
      </div>
    </div>
  </div>
{/if}

<style>
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
</style>
