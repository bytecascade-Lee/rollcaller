<script lang="ts">
  import {STATUS_COLORS, STATUS_DEFAULT_COLOR, statusText} from "$constants/AttendanceStatus";

  let {code, selected = true}: { code: number; selected?: boolean } = $props();
  const style = $derived(STATUS_COLORS[code] ?? STATUS_DEFAULT_COLOR);
  const bg = $derived(selected ? style.bg : style.color);
  const text = $derived(selected ? style.color : style.bg);
</script>

<span
  class="badge"
  style:background={bg}
  style:color={text}
>
  {statusText(code)}
</span>
