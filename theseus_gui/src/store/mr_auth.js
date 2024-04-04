import { ref, onMounted } from 'vue'
import { get as getCredentials, logout as removeCredentials } from '@/helpers/mr_auth.js'
import { handleError } from '@/store/state.js'
import { defineStore } from 'pinia'

export const useModrinthAuth = defineStore('modrinthAuthStore', () => {
  const auth = ref(null)

  const get = async () => {
    try {
      const creds = await getCredentials()
      auth.value = creds
      return creds
    } catch (error) {
      handleError(error)
    }
    return null
  }

  const logout = async () => {
    try {
      const result = await removeCredentials()
      auth.value = null
      return result
    } catch (error) {
      handleError(error)
    }
    return null
  }

  onMounted(() => {
    get()
  })

  return {
    auth,
    get,
    logout,
  }
})
