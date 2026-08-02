<script lang="ts">
  import {statusText} from "$constants/AttendanceStatus";

  let {code}: { code: number } = $props();
</script>

<span class="badge" data-status={code}>
  {statusText(code)}
</span>
