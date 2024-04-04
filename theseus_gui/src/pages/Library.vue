<script setup>
import { onUnmounted, ref } from 'vue'
import GridDisplay from '@/components/GridDisplay.vue'
import { useRoute } from 'vue-router'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { offline_listener, profile_listener } from '@/helpers/events.js'
import { Button, PlusIcon } from 'omorphia'
import InstanceCreationModal from '@/components/ui/InstanceCreationModal.vue'
import { NewInstanceImage } from '@/assets/icons'
import { isOffline } from '@/helpers/utils'
import { useInstances } from '@/store/instances'
import { storeToRefs } from 'pinia'

const route = useRoute()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: 'Library', link: route.path })

const instancesStore = useInstances()
const { instanceList } = storeToRefs(instancesStore)

const offline = ref(await isOffline())
const unlistenOffline = await offline_listener((b) => {
  offline.value = b
})

const unlistenProfile = await profile_listener(async () => {
  await instancesStore.refreshInstances()
})

onUnmounted(() => {
  unlistenProfile()
  unlistenOffline()
})
</script>

<template>
  <GridDisplay v-if="instanceList.length > 0" label="Instances" :instances="instanceList" />
  <div v-else class="no-instance">
    <div class="icon">
      <NewInstanceImage />
    </div>
    <h3>No instances found</h3>
    <Button color="primary" :disabled="offline" @click="$refs.installationModal.show()">
      <PlusIcon />
      Create new instance
    </Button>
    <InstanceCreationModal ref="installationModal" />
  </div>
</template>

<style lang="scss" scoped>
.no-instance {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: var(--gap-md);

  p,
  h3 {
    margin: 0;
  }

  .icon {
    svg {
      width: 10rem;
      height: 10rem;
    }
  }
}
</style>
