<script setup>
import {
  Card,
  Avatar,
  Button,
  formatNumber,
  formatCategory,
  DownloadIcon,
  HeartIcon,
  CalendarIcon,
} from 'omorphia'
import { computed, ref } from 'vue'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useRouter } from 'vue-router'
import { useFetch } from '@/helpers/fetch.js'
import { list } from '@/helpers/profile.js'
import { handleError } from '@/store/notifications.js'
import { install as pack_install } from '@/helpers/pack.js'
dayjs.extend(relativeTime)

const router = useRouter()
const installing = ref(false)

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {}
    },
  },
  confirmModal: {
    type: Object,
    default() {
      return {}
    },
  },
  modInstallModal: {
    type: Object,
    default() {
      return {}
    },
  },
})

const toColor = computed(() => {
  let color = props.project.color

  color >>>= 0
  const b = color & 0xff
  const g = (color >>> 8) & 0xff
  const r = (color >>> 16) & 0xff
  return 'rgba(' + [r, g, b, 1].join(',') + ')'
})

const toTransparent = computed(() => {
  let color = props.project.color

  color >>>= 0
  const b = color & 0xff
  const g = (color >>> 8) & 0xff
  const r = (color >>> 16) & 0xff
  return (
    'linear-gradient(rgba(' +
    [r, g, b, 0.03].join(',') +
    '), 65%, rgba(' +
    [r, g, b, 0.3].join(',') +
    '))'
  )
})

const install = async (e) => {
  e?.stopPropagation()
  installing.value = true
  const versions = await useFetch(
    `https://api.modrinth.com/v2/project/${props.project.project_id}/version`,
    'project versions'
  )

  if (props.project.project_type === 'modpack') {
    const packs = Object.values(await list(true).catch(handleError))

    if (
      packs.length === 0 ||
      !packs
        .map((value) => value.metadata)
        .find((pack) => pack.linked_data?.project_id === props.project.project_id)
    ) {
      installing.value = true
      await pack_install(
        props.project.project_id,
        versions[0].id,
        props.project.title,
        props.project.icon_url
      ).catch(handleError)
      installing.value = false
    } else
      props.confirmModal.show(
        props.project.project_id,
        versions[0].id,
        props.project.title,
        props.project.icon_url
      )
  } else {
    props.modInstallModal.show(props.project.project_id, versions)
  }

  installing.value = false
}
</script>

<template>
  <div class="wrapper">
    <Card class="project-card button-base" @click="router.push(`/project/${project.slug}/`)">
      <div
        class="banner"
        :style="{
          'background-color': project.featured_gallery ?? project.gallery[0] ? null : toColor,
          'background-image': `url(${
            project.featured_gallery ??
            project.gallery[0] ??
            'https://launcher-files.modrinth.com/assets/maze-bg.png'
          })`,
          'no-image': !project.featured_gallery && !project.gallery[0],
        }"
      >
        <div class="badges">
          <div class="badge">
            <DownloadIcon />
            {{ formatNumber(project.downloads) }}
          </div>
          <div class="badge">
            <HeartIcon />
            {{ formatNumber(project.follows) }}
          </div>
          <div class="badge">
            <CalendarIcon />
            {{ formatCategory(dayjs(project.date_modified).fromNow()) }}
          </div>
        </div>
        <div
          class="badges-wrapper"
          :class="{
            'no-image': !project.featured_gallery && !project.gallery[0],
          }"
          :style="{
            background: !project.featured_gallery && !project.gallery[0] ? toTransparent : null,
          }"
        ></div>
      </div>
      <Avatar class="icon" size="sm" :src="project.icon_url" />
      <div class="title">
        <div class="title-text">
          {{ project.title }}
        </div>
        <div class="author">by {{ project.author }}</div>
      </div>
      <div class="description">
        {{ project.description }}
      </div>
    </Card>
    <Button color="primary" class="install" :disabled="installing" @click="install">
      <DownloadIcon />
      {{ installing ? 'Installing' : 'Install' }}
    </Button>
  </div>
</template>

<style scoped lang="scss">
.wrapper {
  position: relative;
  aspect-ratio: 1;
  border-radius: var(--radius-lg);

  &:hover {
    .install:enabled {
      opacity: 1;
    }
  }
}

.project-card {
  display: grid;
  grid-gap: 1rem;
  grid-template:
    '. . . .' 0
    '. icon title .' 3rem
    'banner banner banner banner' auto
    '. description description .' 3.5rem
    '. . . .' 0 / 0 3rem minmax(0, 1fr) 0;
  max-width: 100%;
  height: 100%;
  padding: 0;
  margin: 0;

  .icon {
    grid-area: icon;
  }

  .title {
    max-width: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    grid-area: title;
    white-space: nowrap;

    .title-text {
      width: 100%;
      overflow: hidden;
      text-overflow: ellipsis;
      font-size: var(--font-size-md);
      font-weight: bold;
    }
  }

  .author {
    font-size: var(--font-size-sm);
    grid-area: author;
  }

  .banner {
    grid-area: banner;
    background-size: cover;
    background-position: center;
    position: relative;

    .badges-wrapper {
      width: 100%;
      height: 100%;
      display: flex;
      position: absolute;
      top: 0;
      left: 0;
      mix-blend-mode: hard-light;
    }

    .badges {
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      padding: var(--gap-sm);
      gap: var(--gap-xs);
      display: flex;
      z-index: 10;
      flex-direction: row;
      justify-content: flex-end;
      align-items: flex-end;
    }
  }

  .description {
    grid-area: description;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
  }
}

.badge {
  background-color: var(--color-raised-bg);
  font-size: var(--font-size-xs);
  padding: var(--gap-xs) var(--gap-sm);
  border-radius: var(--radius-sm);

  svg {
    width: 1rem;
    height: 1rem;
    margin-right: var(--gap-xs);
  }
}

.install {
  position: absolute;
  top: calc(5rem + var(--gap-sm));
  right: var(--gap-sm);
  z-index: 10;
  opacity: 0;
  transition: opacity 0.2s ease-in-out;

  svg {
    width: 1rem;
    height: 1rem;
  }
}
</style>
