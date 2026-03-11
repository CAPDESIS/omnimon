<script lang="ts">
  import type { Snippet, ComponentType } from "svelte";

  import Button from "./Button.svelte";

  interface Props {
    icon: ComponentType | string;
    title: string;
    description: string;
    actionLabel?: string;
    onAction?: () => void;
    children?: Snippet;
  }

  let { icon, title, description, actionLabel, onAction, children }: Props = $props();
</script>

<div class="empty-state">
  <div class="empty-icon">
    {#if typeof icon === "string"}
      {@html icon}
    {:else}
      {@const Icon = icon}
      <Icon size={48} />
    {/if}
  </div>
  <h3>{title}</h3>
  <p>{description}</p>
  {#if children}
    <div class="empty-children">
      {@render children()}
    </div>
  {/if}
  {#if actionLabel && onAction}
    <Button class="empty-action" variant="primary" onclick={onAction}>{actionLabel}</Button>
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
    color: var(--fg-muted);
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
    text-align: left;
  }
</style>
