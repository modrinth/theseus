<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { RouterView, RouterLink, useRouter, useRoute } from 'vue-router'
import {
  HomeIcon,
  SearchIcon,
  LibraryIcon,
  SettingsIcon,
  FileIcon,
  Button,
  Notifications,
  XIcon,
  Card,
  TextLogo,
  PlusIcon,
  Avatar,
} from 'omorphia'

import { useLoading, useTheming } from '@/store/state'
import { useInstances } from '@/store/instances'
// import AccountsCard from './components/ui/AccountsCard.vue'
import AccountDropdown from '@/components/ui/platform/AccountDropdown.vue'
import InstanceCreationModal from '@/components/ui/InstanceCreationModal.vue'
import { get } from '@/helpers/settings'
import Breadcrumbs from '@/components/ui/Breadcrumbs.vue'
import RunningAppBar from '@/components/ui/RunningAppBar.vue'
import SplashScreen from '@/components/ui/SplashScreen.vue'
import ModrinthLoadingIndicator from '@/components/modrinth-loading-indicator'
import { handleError, useNotifications } from '@/store/notifications.js'
import { offline_listener, command_listener, warning_listener } from '@/helpers/events.js'
import {
  MinimizeIcon,
  MaximizeIcon,
  ChatIcon,
  ArrowLeftFromLineIcon,
  ArrowRightFromLineIcon,
} from '@/assets/icons'
import { isDev, getOS, isOffline, showLauncherLogsFolder } from '@/helpers/utils.js'
import {
  mixpanel_track,
  mixpanel_init,
  mixpanel_opt_out_tracking,
  mixpanel_is_loaded,
} from '@/helpers/mixpanel.js'
import { useDisableClicks } from '@/composables/click.js'
import { openExternal } from '@/helpers/external.js'
import { await_sync, check_safe_loading_bars_complete } from '@/helpers/state.js'
import { install_from_file } from '@/helpers/pack.js'
import { iconPathAsUrl } from '@/helpers/icon'

import URLConfirmModal from '@/components/ui/URLConfirmModal.vue'
import StickyTitleBar from '@/components/ui/tutorial/StickyTitleBar.vue'
import OnboardingScreen from '@/components/ui/tutorial/OnboardingScreen.vue'

import { saveWindowState, StateFlags } from 'tauri-plugin-window-state-api'
import { getVersion } from '@tauri-apps/api/app'
import { window as TauriWindow } from '@tauri-apps/api'
import { TauriEvent } from '@tauri-apps/api/event'
import { confirm } from '@tauri-apps/api/dialog'
import { type } from '@tauri-apps/api/os'
import { appWindow } from '@tauri-apps/api/window'
import { storeToRefs } from 'pinia'

const themeStore = useTheming()
const urlModal = ref(null)

const isLoading = ref(true)

const videoPlaying = ref(false)
const offline = ref(false)
const showOnboarding = ref(false)
const nativeDecorations = ref(false)

const sidebarOpen = ref(false)

const onboardingVideo = ref()

const failureText = ref(null)
const os = ref('')

const instances = useInstances()
const { instancesByPlayed } = storeToRefs(instances)

defineExpose({
  initialize: async () => {
    isLoading.value = false
    const {
      native_decorations,
      theme,
      opt_out_analytics,
      collapsed_navigation,
      advanced_rendering,
      fully_onboarded,
    } = await get()
    // video should play if the user is not on linux, and has not onboarded
    os.value = await getOS()
    videoPlaying.value = !fully_onboarded && os.value !== 'Linux'
    const dev = await isDev()
    const version = await getVersion()
    showOnboarding.value = !fully_onboarded

    nativeDecorations.value = native_decorations
    if (os.value !== 'MacOS') appWindow.setDecorations(native_decorations)

    themeStore.setThemeState(theme)
    themeStore.collapsedNavigation = collapsed_navigation
    themeStore.advancedRendering = advanced_rendering

    mixpanel_init('014c7d6a336d0efaefe3aca91063748d', { debug: dev, persistence: 'localStorage' })
    if (opt_out_analytics) {
      mixpanel_opt_out_tracking()
    }
    mixpanel_track('Launched', { version, dev, fully_onboarded })

    if (!dev) document.addEventListener('contextmenu', (event) => event.preventDefault())

    if ((await type()) === 'Darwin') {
      document.getElementsByTagName('html')[0].classList.add('mac')
    } else {
      document.getElementsByTagName('html')[0].classList.add('windows')
    }

    offline.value = await isOffline()
    await offline_listener((b) => {
      offline.value = b
    })

    await warning_listener((e) =>
      notificationsWrapper.value.addNotification({
        title: 'Warning',
        text: e.message,
        type: 'warn',
      })
    )

    if (showOnboarding.value) {
      onboardingVideo.value.play()
    }
  },
  failure: async (e) => {
    isLoading.value = false
    failureText.value = e
    os.value = await getOS()
  },
})

const confirmClose = async () => {
  const confirmed = await confirm(
    'An action is currently in progress. Are you sure you want to exit?',
    {
      title: 'Modrinth',
      type: 'warning',
    }
  )
  return confirmed
}

const handleClose = async () => {
  if (failureText.value != null) {
    await TauriWindow.getCurrent().close()
    return
  }
  // State should respond immeiately if it's safe to close
  // If not, code is deadlocked or worse, so wait 2 seconds and then ask the user to confirm closing
  // (Exception: if the user is changing config directory, which takes control of the state, and it's taking a significant amount of time for some reason)
  const isSafe = await Promise.race([
    check_safe_loading_bars_complete(),
    new Promise((r) => setTimeout(r, 2000)),
  ])
  if (!isSafe) {
    const response = await confirmClose()
    if (!response) {
      return
    }
  }
  await await_sync()
  await TauriWindow.getCurrent().close()
}

const openSupport = () => openExternal(window, 'https://support.modrinth.com/')

onMounted(() => {
  return TauriWindow.getCurrent().listen(TauriEvent.WINDOW_CLOSE_REQUESTED, async () => {
    await handleClose()
  })
})

const router = useRouter()
router.afterEach((to, from, failure) => {
  if (mixpanel_is_loaded()) {
    mixpanel_track('PageView', { path: to.path, fromPath: from.path, failed: failure })
  }
})
const route = useRoute()
const isOnBrowse = computed(() => route.path.startsWith('/browse'))

const loading = useLoading()

const notifications = useNotifications()
const notificationsWrapper = ref(null)

watch(notificationsWrapper, () => {
  notifications.setNotifs(notificationsWrapper.value)
})

useDisableClicks(document, window)

// const accounts = ref(null)

command_listener(async (e) => {
  if (e.event === 'RunMRPack') {
    // RunMRPack should directly install a local mrpack given a path
    if (e.path.endsWith('.mrpack')) {
      await install_from_file(e.path).catch(handleError)
      mixpanel_track('InstanceCreate', {
        source: 'CreationModalFileDrop',
      })
    }
  } else {
    // Other commands are URL-based (deep linking)
    urlModal.value.show(e)
  }
})

const toggleSidebar = () => {
  sidebarOpen.value = !sidebarOpen.value
}
</script>

<template>
  <StickyTitleBar v-if="videoPlaying" />
  <video
    v-if="videoPlaying"
    ref="onboardingVideo"
    class="video"
    src="@/assets/video.mp4"
    autoplay
    @ended="videoPlaying = false"
  />
  <div v-if="failureText" class="failure dark-mode">
    <div class="appbar-failure dark-mode">
      <Button v-if="os != 'MacOS'" icon-only @click="TauriWindow.getCurrent().close()">
        <XIcon />
      </Button>
    </div>
    <div class="error-view dark-mode">
      <Card class="error-text">
        <div class="label">
          <h3>
            <span class="label__title size-card-header">Failed to initialize</span>
          </h3>
        </div>
        <div class="error-div">
          Modrinth App failed to load correctly. This may be because of a corrupted file, or because
          the app is missing crucial files.
        </div>
        <div class="error-div">You may be able to fix it one of the following ways:</div>
        <ul class="error-div">
          <li>Ennsuring you are connected to the internet, then try restarting the app.</li>
          <li>Redownloading the app.</li>
        </ul>
        <div class="error-div">
          If it still does not work, you can seek support using the link below. You should provide
          the following error, as well as any recent launcher logs in the folder below.
        </div>
        <div class="error-div">The following error was provided:</div>

        <Card class="error-message">
          {{ failureText.message }}
        </Card>

        <div class="button-row push-right">
          <Button @click="showLauncherLogsFolder"><FileIcon />Open launcher logs</Button>
        </div>
      </Card>
    </div>
  </div>
  <SplashScreen v-else-if="!videoPlaying && isLoading" app-loading />
  <OnboardingScreen v-else-if="showOnboarding" :finish="() => (showOnboarding = false)" />
  <div v-else class="container">
    <div
      class="nav-container"
      data-tauri-drag-region
      :class="`${sidebarOpen ? 'nav-container__open' : ''}`"
      :style="{
        '--sidebar-label-opacity': sidebarOpen ? '1' : '0',
      }"
    >
      <div class="pages-list">
        <div class="square-collapsed-space">
          <Button
            transparent
            icon-only
            class="collapsed-button non-collapse"
            @click="toggleSidebar"
          >
            <ArrowRightFromLineIcon v-if="!sidebarOpen" />
            <ArrowLeftFromLineIcon v-else />
          </Button>
        </div>
      </div>
      <div class="pages-list">
        <!-- <suspense>
          <AccountsCard ref="accounts" mode="small" />
        </suspense> -->
        <div class="pages-list">
          <RouterLink v-tooltip="'Home'" to="/" class="btn icon-only collapsed-button">
            <HomeIcon />
            <span class="collapsed-button__label">Home</span>
          </RouterLink>
          <RouterLink
            v-tooltip="'Browse'"
            to="/browse/modpack"
            class="btn icon-only collapsed-button"
            :class="{
              'router-link-active': isOnBrowse,
            }"
          >
            <SearchIcon />
            <span class="collapsed-button__label">Browse</span>
          </RouterLink>
          <RouterLink v-tooltip="'Library'" to="/library" class="btn icon-only collapsed-button">
            <LibraryIcon />
            <span class="collapsed-button__label">Library</span>
          </RouterLink>
          <suspense>
            <InstanceCreationModal ref="installationModal" />
          </suspense>
        </div>
      </div>
      <div class="divider">
        <hr />
      </div>
      <div class="instances pages-list">
        <RouterLink
          v-for="instance in instancesByPlayed"
          :key="instance.id"
          v-tooltip="instance.metadata.name"
          :to="`/instance/${encodeURIComponent(instance.path)}`"
          class="btn icon-only collapsed-button"
        >
          <Avatar
            class="collapsed-button__icon"
            :src="iconPathAsUrl(instance.metadata?.icon)"
            size="xs"
          />
          <span class="collapsed-button__label">{{ instance.metadata.name }}</span>
        </RouterLink>
      </div>
      <div class="settings pages-list">
        <Button
          v-tooltip="'Get Support'"
          transparent
          icon-only
          class="page-item collapsed-button"
          @click="openSupport"
        >
          <ChatIcon />
          <span class="collapsed-button__label">Support</span>
        </Button>
        <RouterLink v-tooltip="'Settings'" to="/settings" class="btn icon-only collapsed-button">
          <SettingsIcon />
          <span class="collapsed-button__label">Settings</span>
        </RouterLink>
        <Button
          v-tooltip="'Create profile'"
          class="page-item collapsed-button"
          icon-only
          :disabled="offline"
          @click="() => $refs.installationModal.show()"
        >
          <PlusIcon />
          <span class="collapsed-button__label">Create profile</span>
        </Button>
        <AccountDropdown />
      </div>
    </div>
    <div class="view">
      <div class="appbar-row">
        <!-- Top Bar -->
        <div data-tauri-drag-region class="appbar">
          <section class="navigation-controls">
            <router-link :to="'/'">
              <TextLogo class="logo" :animate="false" />
            </router-link>
            <Breadcrumbs after-logo data-tauri-drag-region />
          </section>
          <section class="mod-stats">
            <Suspense>
              <RunningAppBar />
            </Suspense>
          </section>
        </div>
        <section v-if="!nativeDecorations" class="window-controls">
          <Button class="titlebar-button" icon-only @click="() => appWindow.minimize()">
            <MinimizeIcon />
          </Button>
          <Button class="titlebar-button" icon-only @click="() => appWindow.toggleMaximize()">
            <MaximizeIcon />
          </Button>
          <Button
            class="titlebar-button close"
            icon-only
            @click="
              () => {
                saveWindowState(StateFlags.ALL)
                handleClose()
              }
            "
          >
            <XIcon />
          </Button>
        </section>
      </div>
      <div class="router-view">
        <ModrinthLoadingIndicator
          offset-height="var(--appbar-height)"
          :offset-width="sidebarOpen ? 'var(--sidebar-open-width)' : 'var(--sidebar-width)'"
        />
        <RouterView v-slot="{ Component }">
          <template v-if="Component">
            <Suspense @pending="loading.startLoading()" @resolve="loading.stopLoading()">
              <component :is="Component"></component>
            </Suspense>
          </template>
        </RouterView>
      </div>
    </div>
  </div>
  <URLConfirmModal ref="urlModal" />
  <Notifications ref="notificationsWrapper" />
</template>

<style lang="scss" scoped>
.sleek-primary {
  background-color: var(--color-brand-highlight);
  transition: all ease-in-out 0.1s;
}

.logo {
  height: calc(var(--appbar-height) - 2.5rem);
  width: auto;
  min-height: 100%;
  color: var(--color-contrast);
}

.navigation-controls {
  display: flex;
  flex-direction: row;

  align-items: center;
}

.appbar-row {
  display: flex;
  flex-direction: row;
}

.window-controls {
  z-index: 20;
  display: none;
  flex-direction: row;
  align-items: center;

  .titlebar-button {
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all ease-in-out 0.1s;
    background-color: var(--color-raised-bg);
    color: var(--color-base);
    border-radius: 0;
    height: var(--appbar-height);

    &.close {
      &:hover,
      &:active {
        background-color: var(--color-red);
        color: var(--color-accent-contrast);
      }
    }

    &:hover,
    &:active {
      background-color: var(--color-button-bg);
      color: var(--color-contrast);
    }
  }
}

.container {
  --appbar-height: 4.5rem;

  --sidebar-gap: 0.35rem;

  --sidebar-width: 4.5rem;
  --sidebar-open-width: 15rem;
  --sidebar-padding: 0.75rem;

  --sidebar-icon-size: 1.5rem;
  --sidebar-button-size: calc(var(--sidebar-width) - calc(var(--sidebar-padding) * 2));
  --sidebar-open-button-size: calc(var(--sidebar-open-width) - calc(var(--sidebar-padding) * 2));

  height: 100vh;
  display: flex;
  flex-direction: row;
  overflow: hidden;

  .view {
    width: calc(100% - var(--sidebar-width));
    background-color: var(--color-raised-bg);

    .appbar {
      display: flex;
      align-items: center;
      justify-content: space-between;

      flex-grow: 1;
      background: var(--color-raised-bg);
      text-align: center;
      padding: var(--gap-md);
      height: var(--appbar-height);
      gap: var(--gap-sm);
      //no select
      user-select: none;
      -webkit-user-select: none;
    }

    .router-view {
      width: 100%;
      height: calc(100% - var(--appbar-height));
      overflow: auto;
      overflow-x: hidden;
      background-color: var(--color-bg);
      border-top-left-radius: var(--radius-xl);
    }
  }
}

.failure {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: var(--color-bg);

  .appbar-failure {
    display: flex; /* Change to flex to align items horizontally */
    justify-content: flex-end; /* Align items to the right */
    height: var(--appbar-height);
    //no select
    user-select: none;
    -webkit-user-select: none;
  }

  .error-view {
    display: flex; /* Change to flex to align items horizontally */
    justify-content: center;
    width: 100%;
    background-color: var(--color-bg);

    color: var(--color-base);

    .card {
      background-color: var(--color-raised-bg);
    }

    .error-text {
      display: flex;
      max-width: 60%;
      gap: 0.25rem;
      flex-direction: column;

      .error-div {
        // spaced out
        margin: 0.5rem;
      }

      .error-message {
        margin: 0.5rem;
        background-color: var(--color-button-bg);
      }
    }
  }
}

.nav-container {
  display: flex;
  flex-direction: column;

  padding-left: var(--sidebar-padding);
  padding-right: var(--sidebar-padding);
  padding-bottom: var(--sidebar-padding);

  align-items: center;
  justify-content: space-between;

  height: 100%;

  background-color: var(--color-raised-bg);
  box-shadow: var(--shadow-inset-sm), var(--shadow-floating);

  transition: all ease-in-out 0.1s;

  width: var(--sidebar-width);
}

.nav-container__open {
  width: var(--sidebar-open-width);
}

.square-collapsed-space {
  height: var(--appbar-height);
  width: 100%;

  user-select: none;
  -webkit-user-select: none;

  display: flex;
  justify-content: flex-start;
  align-items: center;
}

@media screen and (-webkit-min-device-pixel-ratio: 0) {
  .square-collapsed-space {
    height: auto;
    padding-bottom: var(--gap-md);
  }
}

.divider {
  height: auto;
  width: 100%;

  hr {
    background-color: var(--color-button-bg);
    border: none;
    color: var(--color-button-bg);

    height: 1px;
    width: 100%;

    margin: 0;
  }

  margin-top: var(--sidebar-gap);
  // div should always have + 1 --sidebar-gap margin to the bottom to be equal
  margin-bottom: calc(var(--sidebar-gap) * 2);

  padding-left: var(--sidebar-padding);
  padding-right: var(--sidebar-padding);
}

.instances {
  flex: 1;

  flex-flow: column wrap; // This hides any elements that aren't fully visible
  overflow: hidden;
}

.pages-list {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: flex-start;

  width: 100%;

  gap: var(--sidebar-gap);

  .page-item,
  a {
    display: flex;
    align-items: center;
    word-spacing: 3px;
    background: inherit;
    transition: all ease-in-out 0.1s;
    color: var(--color-base);

    &.router-link-active {
      color: var(--color-brand);
      background: var(--color-brand-highlight);
    }

    &:hover {
      color: var(--color-contrast);
      background: var(--color-button-bg);
    }

    &.router-link-active:hover {
      color: var(--color-brand);
      background: var(--color-brand-highlight);
    }
  }

  &.primary {
    color: var(--color-accent-contrast);
    background-color: var(--color-brand);
  }
}

:deep {
  .non-collapse {
    width: var(--sidebar-button-size) !important;
  }

  .collapsed-button {
    justify-content: flex-start;

    // width: var(--sidebar-icon-size);
    height: var(--sidebar-button-size);
    width: 100%;

    flex-shrink: 0;

    padding: var(--sidebar-padding) !important;
    border-radius: 99999px;
    box-shadow: none;

    white-space: nowrap;
    overflow: hidden;

    transition: all ease-in-out 0.1s;

    .collapsed-button__icon,
    svg {
      width: var(--sidebar-icon-size) !important;
      height: var(--sidebar-icon-size) !important;

      flex-shrink: 0;

      border-radius: var(--radius-xs);
    }

    .collapsed-button__label {
      word-spacing: normal; // Why is this even needed?
      opacity: var(--sidebar-label-opacity);
      transition: all ease-in-out 0.1s;
    }
  }
}

.video {
  margin-top: 2.25rem;
  width: 100vw;
  height: calc(100vh - 2.25rem);
  object-fit: cover;
  border-radius: var(--radius-md);
}

.button-row {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  gap: var(--gap-md);

  .transparent {
    padding: var(--gap-sm) 0;
  }
}
</style>
