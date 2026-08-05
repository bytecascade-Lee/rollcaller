<script lang="ts">
  import {STATUS_COLORS, STATUS_DEFAULT_COLOR, statusText} from "$constants/AttendanceStatus";

  let {code}: { code: number } = $props();
  const style = $derived(STATUS_COLORS[code] ?? STATUS_DEFAULT_COLOR);
</script>

<span
  class="badge"
  style:background={style.bg}
  style:color={style.color}
>
  {statusText(code)}
</span>
