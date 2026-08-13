<script lang="ts">
  let {yes}: { yes: boolean } = $props();
</script>

<div
  class="track"
  class:on={yes}
  aria-hidden="true"
>
  <!-- svelte-ignore element_invalid_self_closing_tag -->
  <div
    class="thumb"
    class:on={yes}
  />
</div>
