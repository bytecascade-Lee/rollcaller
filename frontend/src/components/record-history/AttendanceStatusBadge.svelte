<script lang="ts">
  import {STATUS_COLORS, STATUS_DEFAULT_COLOR, statusText} from "$constants/AttendanceStatus";

  let {code}: { code: number } = $props();
  const style = $derived(STATUS_COLORS[code] ?? STATUS_DEFAULT_COLOR);
</script>

<span
  class="badge"
  style:--badge-bg={style.bg}
  style:--badge-color={style.color}
>
  {statusText(code)}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: var(--app-size-control-sm);
    height: var(--app-size-control-sm);
    padding: 0 var(--app-space-sm);
    border-radius: var(--app-radius-sm);
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
    line-height: 1;
    background: var(--badge-bg, var(--app-color-surface-muted));
    color: var(--badge-color, var(--app-color-text-muted));
  }
</style>
