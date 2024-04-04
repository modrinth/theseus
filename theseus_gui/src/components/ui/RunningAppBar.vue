<template>
  <div class="action-groups">
    <Button
      v-if="currentLoadingBars.length > 0"
      ref="infoButton"
      icon-only
      class="download icon-button"
      @click="toggleCard()"
    >
      <DownloadIcon />
    </Button>
    <div v-if="offline" class="status">
      <span class="circle stopped" />
      <div class="running-text clickable" @click="refreshInternet()">
        <span> Offline </span>
      </div>
    </div>
    <div v-if="selectedProfile" class="status">
      <span class="circle running" />
      <div ref="profileButton" class="running-text">
        <router-link :to="`/instance/${encodeURIComponent(selectedProfile.path)}`">
          {{ selectedProfile.metadata.name }}
        </router-link>
        <div
          v-if="currentProcesses.length > 1"
          class="arrow button-base"
          :class="{ rotate: showProfiles }"
          @click="toggleProfiles()"
        >
          <DropdownIcon />
        </div>
      </div>
      <Button v-tooltip="'Stop instance'" icon-only class="stop icon-button" @click="stop()">
        <StopCircleIcon />
      </Button>
      <Button v-tooltip="'View logs'" icon-only class="utility icon-button" @click="goToTerminal()">
        <TerminalSquareIcon />
      </Button>
      <Button
        v-if="currentLoadingBars.length > 0"
        ref="infoButton"
        icon-only
        class="download icon-button"
        @click="toggleCard()"
      >
        <DownloadIcon />
      </Button>
    </div>
    <div v-else class="status">
      <span class="circle stopped" />
      <span class="running-text"> No instances running </span>
    </div>
  </div>
  <transition name="download">
    <Card v-if="showCard === true && currentLoadingBars.length > 0" ref="card" class="info-card">
      <div v-for="loadingBar in currentLoadingBars" :key="loadingBar.id" class="info-text">
        <h3 class="info-title">
          {{ loadingBar.title }}
        </h3>
        <ProgressBar :progress="Math.floor((100 * loadingBar.current) / loadingBar.total)" />
        <div class="row">
          {{ Math.floor((100 * loadingBar.current) / loadingBar.total) }}% {{ loadingBar.message }}
        </div>
      </div>
    </Card>
  </transition>
  <transition name="download">
    <Card
      v-if="showProfiles === true && currentProcesses.length > 0"
      ref="profiles"
      class="profile-card"
    >
      <Button
        v-for="profile in currentProcesses"
        :key="profile.id"
        class="profile-button"
        @click="selectProfile(profile)"
      >
        <div class="text"><span class="circle running" /> {{ profile.metadata.name }}</div>
        <Button
          v-tooltip="'Stop instance'"
          icon-only
          class="stop icon-button"
          @click.stop="stop(profile.path)"
        >
          <StopCircleIcon />
        </Button>
        <Button
          v-tooltip="'View logs'"
          icon-only
          class="utility icon-button"
          @click.stop="goToTerminal(profile.path)"
        >
          <TerminalSquareIcon />
        </Button>
      </Button>
    </Card>
  </transition>
</template>

<script setup>
import {
  Button,
  DownloadIcon,
  Card,
  StopCircleIcon,
  TerminalSquareIcon,
  DropdownIcon,
} from 'omorphia'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import {
  get_all_running_profiles as getRunningProfiles,
  kill_by_uuid as killProfile,
  get_uuids_by_profile_path as getProfileProcesses,
} from '@/helpers/process'
import { loading_listener, process_listener, offline_listener } from '@/helpers/events'
import { useRouter } from 'vue-router'
import { progress_bars_list } from '@/helpers/state.js'
import { refreshOffline, isOffline } from '@/helpers/utils.js'
import ProgressBar from '@/components/ui/ProgressBar.vue'
import { handleError } from '@/store/notifications.js'
import { mixpanel_track } from '@/helpers/mixpanel'

const router = useRouter()
const card = ref(null)
const profiles = ref(null)
const infoButton = ref(null)
const profileButton = ref(null)
const showCard = ref(false)

const showProfiles = ref(false)

const currentProcesses = ref(await getRunningProfiles().catch(handleError))
const selectedProfile = ref(currentProcesses.value[0])

const offline = ref(await isOffline().catch(handleError))
const refreshInternet = async () => {
  offline.value = await refreshOffline().catch(handleError)
}

const unlistenProcess = await process_listener(async () => {
  await refresh()
})

const unlistenRefresh = await offline_listener(async (b) => {
  offline.value = b
  await refresh()
})

const refresh = async () => {
  currentProcesses.value = await getRunningProfiles().catch(handleError)
  if (!currentProcesses.value.includes(selectedProfile.value)) {
    selectedProfile.value = currentProcesses.value[0]
  }
}

const stop = async (path) => {
  try {
    const processes = await getProfileProcesses(path ?? selectedProfile.value.path)
    await killProfile(processes[0])

    mixpanel_track('InstanceStop', {
      loader: currentProcesses.value[0].metadata.loader,
      game_version: currentProcesses.value[0].metadata.game_version,
      source: 'AppBar',
    })
  } catch (e) {
    console.error(e)
  }
  await refresh()
}

const goToTerminal = (path) => {
  router.push(`/instance/${encodeURIComponent(path ?? selectedProfile.value.path)}/logs`)
}

const currentLoadingBars = ref([])

const refreshInfo = async () => {
  const currentLoadingBarCount = currentLoadingBars.value.length
  currentLoadingBars.value = Object.values(await progress_bars_list().catch(handleError)).map(
    (x) => {
      if (x.bar_type.type === 'java_download') {
        x.title = 'Downloading Java ' + x.bar_type.version
      }
      if (x.bar_type.profile_name) {
        x.title = x.bar_type.profile_name
      }
      if (x.bar_type.pack_name) {
        x.title = x.bar_type.pack_name
      }

      return x
    }
  )
  if (currentLoadingBars.value.length === 0) {
    showCard.value = false
  } else if (currentLoadingBarCount < currentLoadingBars.value.length) {
    showCard.value = true
  }
}

await refreshInfo()
const unlistenLoading = await loading_listener(async () => {
  await refreshInfo()
})

const selectProfile = (profile) => {
  selectedProfile.value = profile
  showProfiles.value = false
}

const handleClickOutsideCard = (event) => {
  const elements = document.elementsFromPoint(event.clientX, event.clientY)
  if (
    card.value &&
    card.value.$el !== event.target &&
    !elements.includes(card.value.$el) &&
    infoButton.value &&
    !infoButton.value.contains(event.target)
  ) {
    showCard.value = false
  }
}

const handleClickOutsideProfile = (event) => {
  const elements = document.elementsFromPoint(event.clientX, event.clientY)
  if (
    profiles.value &&
    profiles.value.$el !== event.target &&
    !elements.includes(profiles.value.$el) &&
    !profileButton.value.contains(event.target)
  ) {
    showProfiles.value = false
  }
}

const toggleCard = async () => {
  showCard.value = !showCard.value
  showProfiles.value = false
  await refreshInfo()
}

const toggleProfiles = async () => {
  if (currentProcesses.value.length === 1) return
  showProfiles.value = !showProfiles.value
  showCard.value = false
}

onMounted(() => {
  window.addEventListener('click', handleClickOutsideCard)
  window.addEventListener('click', handleClickOutsideProfile)
})

onBeforeUnmount(() => {
  window.removeEventListener('click', handleClickOutsideCard)
  window.removeEventListener('click', handleClickOutsideProfile)
  unlistenProcess()
  unlistenLoading()
  unlistenRefresh()
})
</script>

<style scoped lang="scss">
.action-groups {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--gap-md);
}

.arrow {
  transition: transform 0.2s ease-in-out;
  display: flex;
  align-items: center;
  &.rotate {
    transform: rotate(180deg);
  }
}

.status {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.5rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-button-bg);
  padding: var(--gap-sm) var(--gap-lg);
}

.running-text {
  display: flex;
  flex-direction: row;
  gap: var(--gap-xs);
  white-space: nowrap;
  overflow: hidden;
  -webkit-user-select: none; /* Safari */
  -ms-user-select: none; /* IE 10 and IE 11 */
  user-select: none;

  &.clickable:hover {
    cursor: pointer;
  }
}

.circle {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  display: inline-block;
  margin-right: 0.25rem;

  &.running {
    background-color: var(--color-brand);
  }

  &.stopped {
    background-color: var(--color-base);
  }
}

.icon-button {
  background-color: rgba(0, 0, 0, 0);
  box-shadow: none;

  padding: 0 !important;
}

.stop {
  color: var(--color-red);
}

.utility {
  color: var(--color-contrast);
}

.info-card {
  position: absolute;
  top: 3.5rem;
  right: 0.5rem;
  z-index: 9;
  width: 20rem;
  background-color: var(--color-raised-bg);
  box-shadow: var(--shadow-raised);
  display: flex;
  flex-direction: column;
  gap: 1rem;
  overflow: auto;
  transition: all 0.2s ease-in-out;
  border: 1px solid var(--color-button-bg);

  &.hidden {
    transform: translateY(-100%);
  }
}

.loading-option {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
  padding: 0;

  :hover {
    background-color: var(--color-raised-bg-hover);
  }
}

.loading-text {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;

  .row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
  }
}

.loading-icon {
  width: 2.25rem;
  height: 2.25rem;
  display: block;

  :deep(svg) {
    left: 1rem;
    width: 2.25rem;
    height: 2.25rem;
  }
}

.download {
  color: var(--color-brand);
}

.download-enter-active,
.download-leave-active {
  transition: opacity 0.3s ease;
}

.download-enter-from,
.download-leave-to {
  opacity: 0;
}

.progress-bar {
  width: 100%;
}

.info-text {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  margin: 0;
  padding: 0;
}

.info-title {
  margin: 0;
}

.profile-button {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--gap-sm);
  width: 100%;
  background-color: var(--color-raised-bg);
  box-shadow: none;

  .text {
    margin-right: auto;
  }
}

.profile-card {
  position: absolute;
  top: 3.5rem;
  right: 0.5rem;
  z-index: 9;
  background-color: var(--color-raised-bg);
  box-shadow: var(--shadow-raised);
  display: flex;
  flex-direction: column;
  overflow: auto;
  transition: all 0.2s ease-in-out;
  border: 1px solid var(--color-button-bg);
  padding: var(--gap-md);

  &.hidden {
    transform: translateY(-100%);
  }
}

.link {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--gap-sm);
  margin: 0;
  color: var(--color-text);
  text-decoration: none;
}
</style>
