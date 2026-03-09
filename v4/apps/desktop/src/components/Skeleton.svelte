<script lang="ts">
  let {
    width = "100%",
    height = "20px",
    borderRadius = "4px",
    class: className = ""
  }: {
    width?: string;
    height?: string;
    borderRadius?: string;
    class?: string;
  } = $props();
</script>

<div
  class="skeleton {className}"
  style="--sk-width: {width}; --sk-height: {height}; --sk-br: {borderRadius};"
  aria-hidden="true"
></div>

<style>
  .skeleton {
    width: var(--sk-width);
    height: var(--sk-height);
    border-radius: var(--sk-br);
    background: linear-gradient(
      90deg,
      var(--bg-hover) 25%,
      var(--bg-alt) 50%,
      var(--bg-hover) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-loading 1.5s infinite;
  }

  @keyframes skeleton-loading {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
