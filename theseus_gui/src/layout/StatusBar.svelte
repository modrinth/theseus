<script lang="ts">
  import IconChevronLeft from 'virtual:icons/lucide/chevron-left';
  import IconChevronRight from 'virtual:icons/lucide/chevron-right';
  import IconCaretRight from 'virtual:icons/carbon/caret-right';
  import { page } from '$app/stores';

  let path: string[];
  $: path = $page.url.pathname.substring(1).split('/');
</script>

<div class="status-bar">
  <div class="page-nav">
    <button title="Back" on:click={() => window.history.back()}>
      <IconChevronLeft />
    </button>
    <button title="Forward" on:click={() => window.history.forward()}>
      <IconChevronRight />
    </button>
  </div>

  <div class="breadcrumbs">
    {#each path as crumb, index}
      {#if index !== 0}
        <IconCaretRight />
      {/if}
      <a class="breadcrumbs__crumb" href={crumb}>{crumb || 'home'}</a>
    {/each}
  </div>

  <div class="statuses">
    <div>Updating 12 mods...</div>
    <div>236 mods installed</div>
  </div>
</div>

<style lang="postcss">
  .status-bar {
    display: flex;
    padding: 0.75rem;
    grid-gap: 0.75rem;
    background-color: var(--status-bg);
    width: 100%;
    height: 100%;
    align-items: center;
    box-shadow: var(--shadow-raised);

    .page-nav {
      display: flex;
      gap: 2px;

      button {
        display: flex;
      }
    }

    .breadcrumbs {
      display: flex;
      grid-gap: 0.25rem;
      text-transform: capitalize;
      align-items: center;
      overflow-x: auto;

      /* Hide scrollbar */
      &::-webkit-scrollbar {
        display: none; /* Chrome, Safari and Opera */
      }
      -ms-overflow-style: none; /* IE and Edge */
      scrollbar-width: none; /* Firefox */

      &__crumb:hover {
        text-decoration: underline;
      }

      :global(.icon) {
        color: var(--color-text-lightest);
      }
    }

    .statuses {
      margin-left: auto;
      align-items: flex-end;
      justify-content: flex-end;
      display: flex;
      grid-gap: 1rem;
      color: var(--color-text-lightest);
      font-size: 13px;
      line-height: 1.2;
      text-align: right;
    }
  }
</style>
