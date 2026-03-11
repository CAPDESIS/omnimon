<script lang="ts">
  import type { Snippet } from "svelte";

  type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";
  type ButtonSize = "sm" | "md" | "icon";

  interface Props {
    children?: Snippet;
    href?: string;
    variant?: ButtonVariant;
    size?: ButtonSize;
    active?: boolean;
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
    class?: string;
  }

  let {
    children,
    href,
    variant = "secondary",
    size = "md",
    active = false,
    disabled = false,
    type = "button",
    class: className = "",
    ...restProps
  }: Props & Record<string, unknown> = $props();

  let classes = $derived(
    ["ui-button", `ui-button--${variant}`, `ui-button--${size}`, active ? "is-active" : "", className]
      .filter(Boolean)
      .join(" "),
  );
</script>

{#if href}
  <a
    class={classes}
    data-variant={variant}
    data-size={size}
    href={disabled ? undefined : href}
    aria-disabled={disabled ? "true" : undefined}
    tabindex={disabled ? -1 : undefined}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    class={classes}
    data-variant={variant}
    data-size={size}
    type={type}
    disabled={disabled}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}

<style>
  .ui-button {
    --button-bg: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 88%, white 4%);
    --button-bg-hover: color-mix(in srgb, var(--bg-hover) 86%, white 4%);
    --button-border: color-mix(in srgb, var(--border) 82%, var(--accent) 12%);
    --button-color: var(--fg);
    --button-shadow: 0 10px 24px rgba(0, 0, 0, 0.14);
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 36px;
    padding: 0 14px;
    border: 1px solid var(--button-border);
    border-radius: 12px;
    background: var(--button-bg);
    color: var(--button-color);
    font: inherit;
    font-size: calc(var(--base-font-size, 12px) * 0.88);
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.01em;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
    transition:
      transform 0.18s ease,
      background 0.18s ease,
      border-color 0.18s ease,
      color 0.18s ease,
      box-shadow 0.18s ease,
      opacity 0.18s ease;
  }

  .ui-button:hover:not(:disabled):not([aria-disabled="true"]),
  .ui-button:focus-visible {
    transform: translateY(-1px);
    background: var(--button-bg-hover);
    border-color: color-mix(in srgb, var(--button-border) 62%, var(--accent) 38%);
    box-shadow: var(--button-shadow);
    outline: none;
  }

  .ui-button:disabled,
  .ui-button[aria-disabled="true"] {
    filter: grayscale(0.4) brightness(0.7);
    cursor: default;
    pointer-events: none;
    box-shadow: none;
  }

  .ui-button.is-active {
    background: color-mix(in srgb, var(--accent) 25%, var(--bg-surface, var(--bg-alt)) 75%);
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    color: color-mix(in srgb, var(--accent) 70%, white 22%);
  }

  .ui-button--primary {
    --button-bg: linear-gradient(180deg, color-mix(in srgb, var(--accent) 88%, white 10%), var(--accent));
    --button-bg-hover: linear-gradient(180deg, color-mix(in srgb, var(--accent-hover, var(--accent)) 90%, white 8%), var(--accent-hover, var(--accent)));
    --button-border: color-mix(in srgb, var(--accent) 78%, black 12%);
    --button-color: white;
    --button-shadow: 0 14px 26px color-mix(in srgb, var(--accent) 28%, var(--bg));
  }

  .ui-button--secondary {
    --button-bg: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 92%, white 3%);
    --button-bg-hover: color-mix(in srgb, var(--bg-hover) 88%, white 3%);
  }

  .ui-button--danger {
    --button-bg: linear-gradient(180deg, color-mix(in srgb, var(--danger) 88%, white 8%), var(--danger));
    --button-bg-hover: linear-gradient(180deg, color-mix(in srgb, var(--danger-hover, var(--danger)) 90%, white 8%), var(--danger-hover, var(--danger)));
    --button-border: color-mix(in srgb, var(--danger) 82%, black 10%);
    --button-color: white;
    --button-shadow: 0 14px 26px color-mix(in srgb, var(--danger) 22%, var(--bg));
  }

  .ui-button--ghost {
    --button-bg: transparent;
    --button-bg-hover: var(--bg-hover);
    --button-border: var(--border);
    --button-color: var(--fg-dim);
    box-shadow: none;
  }

  .ui-button--sm {
    min-height: 30px;
    padding: 0 12px;
    border-radius: 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.8);
  }

  .ui-button--icon {
    width: 36px;
    min-width: 36px;
    padding: 0;
  }
</style>
