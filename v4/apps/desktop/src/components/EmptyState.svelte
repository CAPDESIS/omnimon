<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    icon: string;        // SVG string o emoji
    title: string;
    description: string;
    actionLabel?: string;
    onAction?: () => void;
    children?: Snippet;
  }

  let { icon, title, description, actionLabel, onAction, children }: Props = $props();
</script>

<div class="empty-state">
  <div class="empty-icon">{@html icon}</div>
  <h3>{title}</h3>
  <p>{description}</p>
  {#if children}
    <div class="empty-children">
      {@render children()}
    </div>
  {/if}
  {#if actionLabel && onAction}
    <button class="empty-action" onclick={onAction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 2rem;
    text-align: center;
    color: var(--text-secondary);
    min-height: 200px;
    flex: 1;
  }
  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.6;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  h3 {
    color: var(--text-primary);
    margin: 0 0 0.5rem;
    font-size: 1.1rem;
  }
  p {
    margin: 0 0 1rem;
    font-size: 0.9rem;
    max-width: 400px;
  }
  .empty-children {
    margin-top: 1rem;
    margin-bottom: 1rem;
    width: 100%;
    display: flex;
    justify-content: center;
  }
  .empty-action {
    padding: 0.5rem 1.5rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }
  .empty-action:hover {
    background: var(--accent-hover);
  }
</style>
