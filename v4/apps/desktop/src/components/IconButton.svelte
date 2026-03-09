<script lang="ts">
  import type { Snippet } from "svelte";

  type IconButtonVariant = "ghost" | "secondary" | "danger";
  type IconButtonSize = "sm" | "md";

  interface Props {
    children?: Snippet;
    variant?: IconButtonVariant;
    size?: IconButtonSize;
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
    class?: string;
    ariaLabel?: string;
    title?: string;
  }

  let {
    children,
    variant = "ghost",
    size = "md",
    disabled = false,
    type = "button",
    class: className = "",
    ariaLabel,
    title,
    ...restProps
  }: Props & Record<string, unknown> = $props();

  let classes = $derived(
    ["ui-icon-button", `ui-icon-button--${variant}`, `ui-icon-button--${size}`, className]
      .filter(Boolean)
      .join(" "),
  );
</script>

<button
  class={classes}
  type={type}
  disabled={disabled}
  aria-label={ariaLabel}
  title={title}
  {...restProps}
>
  {@render children?.()}
</button>

<style>
  .ui-icon-button {
    --icon-button-bg: transparent;
    --icon-button-bg-hover: color-mix(in srgb, var(--bg-hover) 75%, transparent);
    --icon-button-border: color-mix(in srgb, var(--border) 70%, transparent);
    --icon-button-color: var(--fg-dim);
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--icon-button-border);
    border-radius: var(--radius-sm, 4px);
    background: var(--icon-button-bg);
    color: var(--icon-button-color);
    padding: 0;
    cursor: pointer;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      color 0.18s ease,
      transform 0.18s ease,
      box-shadow 0.18s ease;
  }

  .ui-icon-button:hover:not(:disabled),
  .ui-icon-button:focus-visible {
    background: var(--icon-button-bg-hover);
    color: var(--fg);
    border-color: color-mix(in srgb, var(--icon-button-border) 55%, var(--accent) 45%);
    transform: translateY(-1px);
    outline: none;
  }

  .ui-icon-button:disabled {
    opacity: 0.45;
    cursor: default;
    pointer-events: none;
  }

  .ui-icon-button--ghost {
    --icon-button-border: transparent;
  }

  .ui-icon-button--secondary {
    --icon-button-bg: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 92%, white 3%);
    --icon-button-bg-hover: color-mix(in srgb, var(--bg-hover) 88%, white 3%);
    --icon-button-border: var(--border);
  }

  .ui-icon-button--danger {
    --icon-button-bg: color-mix(in srgb, var(--danger) 18%, transparent);
    --icon-button-bg-hover: color-mix(in srgb, var(--danger) 26%, transparent);
    --icon-button-border: color-mix(in srgb, var(--danger) 48%, var(--border));
    --icon-button-color: var(--danger);
  }

  .ui-icon-button--sm {
    width: 18px;
    min-width: 18px;
    height: 18px;
    font-size: 12px;
  }

  .ui-icon-button--md {
    width: 30px;
    min-width: 30px;
    height: 30px;
    font-size: 18px;
  }
</style>
