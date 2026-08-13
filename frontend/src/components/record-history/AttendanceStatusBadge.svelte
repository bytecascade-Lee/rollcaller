<script lang="ts">

  import {attendanceStatusStore} from "$stores/attendanceStatusStore.svelte";

  let {id = 0, selected = true}: { id: number; selected?: boolean } = $props();
  const attendanceStatus = $derived(attendanceStatusStore.attendanceStatus(id));
  const background = $derived(selected ? attendanceStatus.background : attendanceStatus.color);
  const color = $derived(selected ? attendanceStatus.color : attendanceStatus.background);
</script>

<span
  class="badge"
  style:background={background}
  style:color={color}
>
  {attendanceStatus.name}
</span>
