<script setup>
import { ref, onUnmounted, shallowRef, computed } from 'vue'
import { useRoute } from 'vue-router'
import RowDisplay from '@/components/RowDisplay.vue'
import { list } from '@/helpers/profile.js'
import { offline_listener, profile_listener } from '@/helpers/events'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useFetch } from '@/helpers/fetch.js'
import { handleError } from '@/store/notifications.js'
import dayjs from 'dayjs'
import { isOffline } from '@/helpers/utils'

const featuredModpacks = ref({})
const featuredMods = ref({})
const filter = ref('')

const route = useRoute()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: 'Home', link: route.path })

const recentInstances = shallowRef([])

const offline = ref(await isOffline())

const getInstances = async () => {
  const profiles = await list(true).catch(handleError)
  recentInstances.value = Object.values(profiles).sort((a, b) => {
    return dayjs(b.metadata.last_played ?? 0).diff(dayjs(a.metadata.last_played ?? 0))
  })

  let filters = []
  for (const instance of recentInstances.value) {
    if (instance.metadata.linked_data?.modrinth_modpack?.project_id) {
      filters.push(
        `NOT"project_id"="${instance.metadata.linked_data?.modrinth_modpack?.project_id}"`
      )
    }
  }
  filter.value = filters.join(' AND ')
}

const getFeaturedModpacks = async () => {
  const response = await useFetch(
    `https://staging-api.modrinth.com/v2/search?facets=[["project_type:modpack"]]&limit=10&index=follows&filters=${filter.value}`,
    'featured modpacks',
    offline.value
  )
  if (response) {
    featuredModpacks.value = response.hits
  } else {
    featuredModpacks.value = []
  }
}
const getFeaturedMods = async () => {
  const response = await useFetch(
    'https://staging-api.modrinth.com/v2/search?facets=[["project_type:mod"]]&limit=10&index=follows',
    'featured mods',
    offline.value
  )
  if (response) {
    featuredMods.value = response.hits
  } else {
    featuredModpacks.value = []
  }
}

await getInstances()

await Promise.all([getFeaturedModpacks(), getFeaturedMods()])

const unlistenProfile = await profile_listener(async (e) => {
  await getInstances()
  if (e.event === 'created' || e.event === 'removed') {
    await Promise.all([getFeaturedModpacks(), getFeaturedMods()])
  }
})

const unlistenOffline = await offline_listener(async (b) => {
  offline.value = b
  if (!b) {
    await Promise.all([getFeaturedModpacks(), getFeaturedMods()])
  }
})

// computed sums of recentInstances, featuredModpacks, featuredMods, treating them as arrays if they are not
const total = computed(() => {
  return (
    (recentInstances.value?.length ?? 0) +
    (featuredModpacks.value?.length ?? 0) +
    (featuredMods.value?.length ?? 0)
  )
})

onUnmounted(() => {
  unlistenProfile()
  unlistenOffline()
})
</script>

<template>
  <div class="page-container">
    <RowDisplay
      v-if="total > 0"
      :instances="[
        {
          label: 'Jump back in',
          route: '/library',
          instances: recentInstances,
          downloaded: true,
        },
        {
          label: 'Popular packs',
          route: '/browse/modpack',
          instances: featuredModpacks,
          downloaded: false,
        },
        {
          label: 'Popular mods',
          route: '/browse/mod',
          instances: featuredMods,
          downloaded: false,
        },
      ]"
      :can-paginate="true"
    />
  </div>
</template>

<style lang="scss" scoped>
.page-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 100%;
}
</style>
