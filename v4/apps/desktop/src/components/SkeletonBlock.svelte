<script lang="ts">
  interface Props {
    width?: string;
    height?: string;
    rounded?: string;
    className?: string;
  }

  let {
    width = "100%",
    height = "1rem",
    rounded = "10px",
    className = "",
  }: Props = $props();
</script>

<span
  class={`skeleton-block ${className}`}
  aria-hidden="true"
  style={`--skeleton-width:${width};--skeleton-height:${height};--skeleton-radius:${rounded};`}
></span>

<style>
  .skeleton-block {
    display: block;
    width: var(--skeleton-width);
    height: var(--skeleton-height);
    border-radius: var(--skeleton-radius);
    background:
      linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.16), transparent),
      color-mix(in srgb, var(--border) 72%, transparent);
    background-size: 220px 100%, 100% 100%;
    animation: skeleton-shimmer 1.15s linear infinite;
  }

  @keyframes skeleton-shimmer {
    from {
      background-position: -220px 0, 0 0;
    }
    to {
      background-position: 220px 0, 0 0;
    }
  }
 </style>
