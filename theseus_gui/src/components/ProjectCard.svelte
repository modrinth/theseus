<script lang="ts">
  import { t } from 'svelte-intl-precompile';
  import IconHeart from 'virtual:icons/lucide/heart';
  import IconDownload from 'virtual:icons/heroicons-outline/download';
  import IconCalendar from 'virtual:icons/lucide/calendar';
  import { ago, simplify } from 'omorphia/utils';
  import { Avatar, Button } from 'omorphia';
  import { tagIcons } from '$generated/tags.json';

  export let project;

  // @ts-ignore: Author is only available in the result
  let author = project.author ?? '';

  // @ts-ignore: ID is in different locations in the result and project
  let id = project.id ?? project.project_id;

  // @ts-ignore: Updated is in different locations in the result and project
  let updated = project.date_modified ?? project.updated;

  const href = `/${project.project_type}/${project.slug || id}`;
</script>

<div class="card project-card">
  <a {href} tabindex="-1">
    <Avatar src={project.icon_url} size="md" />
  </a>

  <div class="project-card__info">
    <div class="project-card__info__top">
      <div class="project-card__info__top__text">
        <span
          ><a class="project-card__info__top__text__title" {href}>{project.title}</a>
          {#if author}
            <a href="/user/{author}" class="project-card__info__top__text__author"
              >{@html $t('generic.byline', { values: { author } })}</a
            >
          {/if}
        </span>

        <p class="summary">
          {project.description}
        </p>
      </div>
      <div class="button-group">
        <Button color="primary"><IconDownload />Install</Button>
      </div>
    </div>

    <div class="tag-group">
      {#each project.categories as category}
        <div class="tag">
          {@html tagIcons[category] || '?'}
          {$t(`tags.${category}`)}
        </div>
      {/each}

      <span class="stat">
        <IconDownload />
        <b>{simplify(project.downloads)}</b>
      </span>
      <span class="tag">
        <IconCalendar />
        {@html $t('stats.updated', { values: { ago: ago(updated) } })}
      </span>
    </div>
  </div>

  {#if !$$slots.actions}{:else}
    <slot name="actions" />
  {/if}
</div>

<style lang="postcss">
  .project-card {
    flex-direction: row;
    flex-wrap: nowrap;
    padding: 10px;
    grid-gap: 1rem;

    &__info {
      display: flex;
      flex-direction: column;
      grid-gap: 0.25rem;
      line-height: 100%;
      margin-top: 0.2rem;
      width: 100%;

      &__top {
        display: flex;
        gap: 16px;

        &__text {
          display: flex;
          flex-direction: column;
          gap: 4px;

          &__title {
            font-weight: bold;
            font-size: 16px;
          }

          &__author {
            white-space: nowrap;
            font-size: 12px;
          }

          .summary {
            text-overflow: ellipsis;
            overflow: hidden;
            white-space: nowrap;
            width: calc(100vw - 224px - 24px - 16px - 64px - 130px);
          }
        }

        :global(.button-group) {
          margin-left: auto;
          height: 33px;
        }
      }

      .tag-group {
        .stat {
          margin-left: auto;
        }
      }
    }
  }
</style>
