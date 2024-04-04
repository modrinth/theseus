import { ref, onMounted, computed } from 'vue'
import dayjs from 'dayjs'

import { list } from '@/helpers/profile.js'
import { handleError } from '@/store/notifications'
import { defineStore } from 'pinia'

export const useInstances = defineStore('instancesStore', () => {
  const instances = ref({})

  const instanceList = computed(() => {
    return Object.values(instances.value)
  })
  const instancesByPlayed = computed(() => {
    return instanceList.value.sort((a, b) => {
      return dayjs(b?.metadata?.last_played ?? 0).diff(dayjs(a?.metadata?.last_played ?? 0))
    })
  })

  const setInstances = async () => {
    try {
      const p = await list(true)
      instances.value = p
    } catch (error) {
      handleError(error)
    }
  }

  onMounted(async () => {
    await setInstances()
  })

  const refreshInstances = async () => {
    await setInstances()
  }

  return {
    instanceList,
    instancesByPlayed,

    refreshInstances,
  }
})
