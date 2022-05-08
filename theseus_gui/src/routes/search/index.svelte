<script lang="ts" context="module">
  /** @type {import('./index').Load} */
  export async function load({ fetch }) {
    const response = await fetch(`https://api.modrinth.com/v2/search?query=&limit=10&offset=0&index=relevance`);

    return {
      props: {
        projects: response.ok && (await response.json()).hits
      }
    };
  }
</script>

<script lang="ts">
  import IconSearch from 'virtual:icons/heroicons-outline/search';
  import IconSortDescending from 'virtual:icons/heroicons-outline/sort-descending';
  import IconBox from 'virtual:icons/lucide/box';
  import IconGlobe from 'virtual:icons/lucide/globe';
  import IconCpu from 'virtual:icons/lucide/cpu';
  import IconTruck from 'virtual:icons/lucide/truck';
  import IconFileText from 'virtual:icons/lucide/file-text';

  import { TextInput, Button } from 'omorphia';
  import ProjectCard from '$components/ProjectCard.svelte';

  export let projects;
</script>

<div class="controls">
  <div class="controls__row">
    <TextInput placeholder="Search..." icon={IconSearch} />
    <Button color="tertiary"><IconSortDescending />Sort by relevance</Button>
  </div>
  <div class="controls__row controls__row--overflow">
    <Button color="secondary"><IconBox />Minecraft versions</Button>
    <Button color="secondary"><IconGlobe />Categories</Button>
    <Button color="secondary"><IconCpu />Environment</Button>
    <Button color="secondary"><IconTruck />Mod loaders</Button>
    <Button color="secondary"><IconFileText />License</Button>
  </div>
</div>

<div class="results">
  {#each projects as project}
    <ProjectCard {project} />
  {/each}
</div>

<style lang="postcss">
  .controls {
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 10px;

    &__row {
      display: flex;
      gap: 8px;

      &--overflow {
        overflow-x: auto;
        margin: 0px -4px;
        padding: 0 6px;
        width: calc(100% + 3px);
        mask-image: linear-gradient(to right, transparent, hsla(0, 0%, 0%, 1) 1% 99%, transparent);

        /* Hide scrollbar */
        -ms-overflow-style: none;
        scrollbar-width: none;
        &::-webkit-scrollbar {
          display: none;
        }
      }

      :global(.text-input) {
        flex: 1 1;
      }

      :global(.text-input > input) {
        width: auto;
      }
    }
  }

  .results {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
</style>
