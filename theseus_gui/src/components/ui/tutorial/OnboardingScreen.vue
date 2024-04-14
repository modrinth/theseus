<script setup>
import {
  Button,
  HomeIcon,
  SearchIcon,
  LibraryIcon,
  PlusIcon,
  SettingsIcon,
  XIcon,
  Notifications,
} from 'omorphia'
import { appWindow } from '@tauri-apps/api/window'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import Breadcrumbs from '@/components/ui/Breadcrumbs.vue'
import FakeAppBar from '@/components/ui/tutorial/FakeAppBar.vue'
import FakeAccountsCard from '@/components/ui/tutorial/FakeAccountsCard.vue'
import { MinimizeIcon, MaximizeIcon } from '@/assets/icons'
import ModrinthLoadingIndicator from '@/components/modrinth-loading-indicator.js'
import FakeSearch from '@/components/ui/tutorial/FakeSearch.vue'
import FakeGridDisplay from '@/components/ui/tutorial/FakeGridDisplay.vue'
import FakeRowDisplay from '@/components/ui/tutorial/FakeRowDisplay.vue'
import { onMounted, ref } from 'vue'
import { window } from '@tauri-apps/api'
import TutorialTip from '@/components/ui/tutorial/TutorialTip.vue'
import FakeSettings from '@/components/ui/tutorial/FakeSettings.vue'
import { get, set } from '@/helpers/settings.js'
import mixpanel from 'mixpanel-browser'
import GalleryImage from '@/components/ui/tutorial/GalleryImage.vue'
import LoginCard from '@/components/ui/tutorial/LoginCard.vue'
import StickyTitleBar from '@/components/ui/tutorial/StickyTitleBar.vue'
import { auto_install_java, get_jre } from '@/helpers/jre.js'
import { handleError } from '@/store/notifications.js'
import ImportingCard from '@/components/ui/tutorial/ImportingCard.vue'
import ModrinthLoginScreen from '@/components/ui/tutorial/ModrinthLoginScreen.vue'
import PreImportScreen from '@/components/ui/tutorial/PreImportScreen.vue'

const phase = ref(0)
const page = ref(1)

const props = defineProps({
  finish: {
    type: Function,
    default: () => {},
  },
})

const flow = ref('')

const nextPhase = () => {
  phase.value++
  mixpanel.track('TutorialPhase', { page: phase.value })
}

const prevPhase = () => {
  phase.value--
}

const nextPage = (newFlow) => {
  page.value++
  mixpanel.track('OnboardingPage', { page: page.value })

  if (newFlow) {
    flow.value = newFlow
  }
}

const endOnboarding = () => {
  nextPhase()
}

const prevPage = () => {
  page.value--
}

const finishOnboarding = async () => {
  mixpanel.track('OnboardingFinish')
  const settings = await get()
  settings.fully_onboarded = true
  await set(settings)
  props.finish()
}

async function fetchSettings() {
  const fetchSettings = await get().catch(handleError)
  if (!fetchSettings.java_globals) {
    fetchSettings.java_globals = {}
  }

  if (!fetchSettings.java_globals.JAVA_17) {
    const path1 = await auto_install_java(17).catch(handleError)
    fetchSettings.java_globals.JAVA_17 = await get_jre(path1).catch(handleError)
  }

  if (!fetchSettings.java_globals.JAVA_8) {
    const path2 = await auto_install_java(8).catch(handleError)
    fetchSettings.java_globals.JAVA_8 = await get_jre(path2).catch(handleError)
  }

  await set(fetchSettings).catch(handleError)
}

onMounted(async () => {
  await fetchSettings()
})
</script>

<template>
  <div v-if="phase === 0" class="onboarding">
    <StickyTitleBar />
    <GalleryImage
      v-if="page === 1"
      :gallery="[
        {
          url: 'https://cdn.discordapp.com/attachments/817413688771608587/1131109353928265809/Screenshot_2023-07-15_at_4.16.18_PM.png',
          title: 'Discovery',
          subtitle: 'See the latest and greatest mods and modpacks to play with from Modrinth',
        },
        {
          url: 'https://cdn.discordapp.com/attachments/817413688771608587/1131109354238640238/Screenshot_2023-07-15_at_4.17.43_PM.png',
          title: 'Profile Management',
          subtitle:
            'Play, manage and search through all the amazing profiles downloaded on your computer at any time, even offline!',
        },
      ]"
      logo
    >
      <Button color="primary" @click="nextPage"> Get started </Button>
    </GalleryImage>
    <LoginCard v-else-if="page === 2" :next-page="nextPage" :prev-page="prevPage" />
    <ModrinthLoginScreen
      v-else-if="page === 3"
      :modal="false"
      :next-page="nextPage"
      :prev-page="prevPage"
      :flow="flow"
    />
    <PreImportScreen
      v-else-if="page === 4"
      :next-page="endOnboarding"
      :prev-page="prevPage"
      :import-page="nextPage"
    />
    <ImportingCard v-else-if="page === 5" :next-page="endOnboarding" :prev-page="prevPage" />
  </div>
  <div v-else class="container">
    <StickyTitleBar v-if="phase === 9" />
    <div v-if="phase < 9" class="nav-container">
      <div class="nav-section">
        <FakeAccountsCard :show-demo="phase === 3">
          <TutorialTip
            :progress-function="nextPhase"
            :previous-function="prevPhase"
            :progress="phase"
            title="Signing in"
            description="The Modrinth App uses your Microsoft account to allow you to launch Minecraft. You can sign in with your Microsoft account here, and switch between multiple accounts."
          />
        </FakeAccountsCard>
        <div class="pages-list">
          <div class="btn icon-only" :class="{ active: phase < 4 }">
            <HomeIcon />
          </div>
          <div
            class="btn icon-only"
            :class="{ active: phase === 4 || phase === 5, highlighted: phase === 4 }"
          >
            <SearchIcon />
          </div>
          <div
            class="btn icon-only"
            :class="{
              active: phase === 6 || phase === 7,
              highlighted: phase === 6,
            }"
          >
            <LibraryIcon />
          </div>
        </div>
      </div>
      <div class="settings pages-list">
        <Button class="sleek-primary" icon-only>
          <PlusIcon />
        </Button>
        <Button icon-only :class="{ active: phase === 8, highlighted: phase === 8 }">
          <SettingsIcon />
        </Button>
      </div>
    </div>
    <div v-if="phase < 9" class="view">
      <div data-tauri-drag-region class="appbar">
        <section class="navigation-controls">
          <Breadcrumbs data-tauri-drag-region />
        </section>
        <section class="mod-stats">
          <FakeAppBar
            :show-running="phase === 7"
            :show-download="phase === 5"
            :exit="finishOnboarding"
          >
            <template #running>
              <TutorialTip
                :progress-function="nextPhase"
                :previous-function="prevPhase"
                :progress="phase"
                title="Playing modpacks"
                description="When you launch a modpack, you can manage it directly in the title bar here. You can stop the modpack, view the logs, and see all currently running packs."
              />
            </template>
            <template #download>
              <TutorialTip
                :progress-function="nextPhase"
                :previous-function="prevPhase"
                :progress="phase"
                title="Installing modpacks"
                description="When you download a modpack, Modrinth App will automatically install it for you. You can view the progress of the installation here."
              />
            </template>
          </FakeAppBar>
        </section>
        <section class="window-controls">
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
                window.getCurrent().close()
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
          offset-width="var(--sidebar-width)"
        />
        <Notifications ref="notificationsWrapper" />
        <FakeRowDisplay v-if="phase < 4 || phase > 8" :show-instance="phase === 2" />
        <FakeGridDisplay v-if="phase === 6 || phase === 7" :show-instances="phase === 6" />
        <suspense>
          <FakeSearch v-if="phase === 4 || phase === 5" :show-search="phase === 4" />
        </suspense>
        <FakeSettings v-if="phase === 8" />
      </div>
    </div>
    <TutorialTip
      v-if="phase === 1"
      class="first-tip highlighted"
      :progress-function="nextPhase"
      :progress="phase"
      title="Enter the Modrinth App!"
      description="This is the Modrinth App guide. Key parts are marked with a green shadow. Click 'Next' to
      proceed. You can leave the tutorial anytime using the Exit button above the plus button on the bottom left."
    />
    <div v-if="phase === 1" class="whole-page-shadow" />
    <TutorialTip
      v-if="phase === 2"
      class="sticky-tip"
      :progress-function="nextPhase"
      :previous-function="prevPhase"
      :progress="phase"
      title="Home page"
      description="This is the home page. Here you can see all the latest modpacks, mods, and other content on Modrinth. You can also see a few of your installed modpacks here."
    />
    <TutorialTip
      v-if="phase === 4"
      class="sticky-tip"
      :progress-function="nextPhase"
      :previous-function="prevPhase"
      :progress="phase"
      title="Searching for content"
      description="You can search for content on Modrinth by navigating to the search page. You can search for mods, modpacks, and more, and install them directly from here."
    />
    <TutorialTip
      v-if="phase === 6"
      class="sticky-tip"
      :progress-function="nextPhase"
      :previous-function="prevPhase"
      :progress="phase"
      title="Modpack library"
      description="You can view all your installed modpacks in the library. You can launch any modpack from here, or click the card to view more information about it."
    />
    <TutorialTip
      v-if="phase === 8"
      class="sticky-tip"
      :progress-function="nextPhase"
      :previous-function="prevPhase"
      :progress="phase"
      title="Settings"
      description="You will be able to view and change the settings for the Modrinth App here. You can change the appearance, set and download new Java versions, and more."
    />
    <TutorialTip
      v-if="phase === 9"
      class="final-tip highlighted"
      :progress-function="finishOnboarding"
      :progress="phase"
      title="Enter the Modrinth App!"
      description="That's it! You're ready to use the Modrinth App. If you need help, you can always join our discord server!"
    />
  </div>
</template>

<style scoped lang="scss">
.sleek-primary {
  background-color: var(--color-brand-highlight);
  transition: all ease-in-out 0.1s;
}

.navigation-controls {
  flex-grow: 1;
  width: min-content;
}

.window-controls {
  z-index: 20;
  display: none;
  flex-direction: row;
  align-items: center;
  gap: 0.25rem;

  .titlebar-button {
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all ease-in-out 0.1s;
    background-color: var(--color-raised-bg);
    color: var(--color-base);

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
  --appbar-height: 3.25rem;
  --sidebar-width: 4.5rem;

  height: 100vh;
  display: flex;
  flex-direction: row;
  overflow: hidden;

  .view {
    width: calc(100% - var(--sidebar-width));

    .appbar {
      display: flex;
      align-items: center;
      background: var(--color-raised-bg);
      box-shadow: var(--shadow-inset-sm), var(--shadow-floating);
      padding: var(--gap-md);
      height: 3.25rem;
      gap: var(--gap-sm);
      user-select: none;
      -webkit-user-select: none;
    }

    .router-view {
      width: 100%;
      height: calc(100% - 3.125rem);
      overflow: auto;
      overflow-x: hidden;
      background-color: var(--color-bg);
    }
  }
}

.nav-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  height: 100%;
  background-color: var(--color-raised-bg);
  box-shadow: var(--shadow-inset-sm), var(--shadow-floating);
  padding: var(--gap-md);
  width: var(--sidebar-width);
  max-width: var(--sidebar-width);
  min-width: var(--sidebar-width);

  --sidebar-width: 4.5rem;
}

.pages-list {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: flex-start;
  width: 100%;
  gap: 0.5rem;

  .btn {
    background-color: var(--color-raised-bg);
    height: 3rem !important;
    width: 3rem !important;
    padding: 0.75rem;
    border-radius: var(--radius-md);
    box-shadow: none;

    svg {
      width: 1.5rem !important;
      height: 1.5rem !important;
      max-width: 1.5rem !important;
      max-height: 1.5rem !important;
    }

    &.active {
      background-color: var(--color-button-bg);
      box-shadow: var(--shadow-floating);
    }

    &.sleek-primary {
      background-color: var(--color-brand-highlight);
      transition: all ease-in-out 0.1s;
    }
  }
}

.nav-section {
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  align-items: center;
  width: 100%;
  height: 100%;
  gap: 1rem;
}

.sticky-tip {
  position: absolute;
  bottom: 1rem;
  right: 1rem;
  z-index: 10;
}

.intro-card {
  display: flex;
  flex-direction: column;
  padding: var(--gap-xl);

  .app-logo {
    width: 100%;
    height: auto;
    display: block;
    margin: auto;
  }

  p {
    color: var(--color-contrast);
    text-align: left;
    width: 100%;
  }

  .actions {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: flex-end;
    gap: var(--gap-sm);
  }
}

.final-tip {
  position: absolute;
  bottom: 50%;
  right: 50%;
  transform: translate(50%, 50%);
  z-index: 10;
}

.onboarding {
  background:
    top linear-gradient(0deg, #31375f, rgba(8, 14, 55, 0)),
    url(https://cdn.modrinth.com/landing-new/landing-lower.webp);
  background-size: cover;
  height: 100vh;
  min-height: 100vh;
  max-height: 100vh;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--gap-xl);
  padding-top: calc(2.5rem + var(--gap-lg));
}

.first-tip {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 10;
}

.whole-page-shadow {
  position: absolute;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100%;
  backdrop-filter: brightness(0.5);
  -webkit-backdrop-filter: brightness(0.5);
  z-index: 9;
}
</style>
