<script setup>
import { SearchIcon, PlusIcon } from 'omorphia'
import { onUnmounted, shallowRef } from 'vue'
import { useRoute } from 'vue-router'
import RowDisplay from '@/components/RowDisplay.vue'
import { list } from '@/helpers/profile.js'
import { profile_listener } from '@/helpers/events'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { handleError } from '@/store/notifications.js'
import dayjs from 'dayjs'

const route = useRoute()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: 'Home', link: route.path })

const recentInstances = shallowRef([])

const getInstances = async () => {
  const profiles = await list(true).catch(handleError)
  recentInstances.value = Object.values(profiles).sort((a, b) => {
    return dayjs(b.metadata.last_played ?? 0).diff(dayjs(a.metadata.last_played ?? 0))
  })
}

await getInstances()

const unlistenProfile = await profile_listener(async () => {
  await getInstances()
})

onUnmounted(() => {
  unlistenProfile()
})
</script>

<template>
  <div class="page-container">
    <RowDisplay
      v-if="recentInstances.length > 0"
      :instances="[
        {
          label: 'Jump back in',
          route: '/library',
          instances: recentInstances,
          downloaded: true,
        },
      ]"
      :can-paginate="true"
    />
    <div v-else class="modrinth-intro">
      <h1>Welcome to the Modrinth app!</h1>
      <p>
        Thank you for downloading. Manage your mods, browse Modrinth, and play Minecraft all in one
        place!
      </p>
      <router-link class="btn" to=""><SearchIcon /> Browse content</router-link>
      <router-link class="btn" to=""><PlusIcon /> Create a profile</router-link>
      <router-link class="btn" to="">Import from other launchers</router-link>
    </div>
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

.modrinth-intro {
}
</style>
