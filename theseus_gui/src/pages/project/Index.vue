<template>
  <div class="root-container">
    <div v-if="data" class="project-sidebar">
      <Card v-if="instance" class="small-instance">
        <router-link class="instance" :to="`/instance/${encodeURIComponent(instance.path)}`">
          <Avatar
            :src="
              !instance.metadata.icon ||
              (instance.metadata.icon && instance.metadata.icon.startsWith('http'))
                ? instance.metadata.icon
                : convertFileSrc(instance.metadata?.icon)
            "
            :alt="instance.metadata.name"
            size="sm"
          />
          <div class="small-instance_info">
            <span class="title">{{
              instance.metadata.name.length > 20
                ? instance.metadata.name.substring(0, 20) + '...'
                : instance.metadata.name
            }}</span>
            <span>
              {{
                instance.metadata.loader.charAt(0).toUpperCase() + instance.metadata.loader.slice(1)
              }}
              {{ instance.metadata.game_version }}
            </span>
          </div>
        </router-link>
      </Card>
      <Card class="sidebar-card" @contextmenu.prevent.stop="handleRightClick">
        <Avatar size="lg" :src="data.icon_url" />
        <div class="instance-info">
          <h2 class="name">{{ data.title }}</h2>
          {{ data.description }}
        </div>
        <Categories
          class="tags"
          :categories="
            categories.filter(
              (cat) => data.categories.includes(cat.name) && cat.project_type === 'mod'
            )
          "
          type="ignored"
        >
          <EnvironmentIndicator
            :client-side="data.client_side"
            :server-side="data.server_side"
            :type="data.project_type"
          />
        </Categories>
        <hr class="card-divider" />
        <div class="button-group">
          <Button
            color="primary"
            class="instance-button"
            :disabled="installed === true || installing === true"
            @click="install(null)"
          >
            <DownloadIcon v-if="!installed && !installing" />
            <CheckIcon v-else-if="installed" />
            {{ installing ? 'Installing...' : installed ? 'Installed' : 'Install' }}
          </Button>
          <a
            :href="`https://modrinth.com/${data.project_type}/${data.slug}`"
            rel="external"
            class="btn"
          >
            <ExternalIcon />
            Site
          </a>
        </div>
        <hr class="card-divider" />
        <div class="stats">
          <div class="stat">
            <DownloadIcon aria-hidden="true" />
            <p>
              <strong>{{ formatNumber(data.downloads) }}</strong>
              <span class="stat-label"> download<span v-if="data.downloads !== '1'">s</span></span>
            </p>
          </div>
          <div class="stat">
            <HeartIcon aria-hidden="true" />
            <p>
              <strong>{{ formatNumber(data.followers) }}</strong>
              <span class="stat-label"> follower<span v-if="data.followers !== '1'">s</span></span>
            </p>
          </div>
          <div class="stat date">
            <CalendarIcon aria-hidden="true" />
            <span
              ><span class="date-label">Created </span> {{ dayjs(data.published).fromNow() }}</span
            >
          </div>
          <div class="stat date">
            <UpdatedIcon aria-hidden="true" />
            <span
              ><span class="date-label">Updated </span> {{ dayjs(data.updated).fromNow() }}</span
            >
          </div>
        </div>
        <hr class="card-divider" />
        <div class="button-group">
          <Button class="instance-button" disabled>
            <ReportIcon />
            Report
          </Button>
          <Button class="instance-button" disabled>
            <HeartIcon />
            Follow
          </Button>
        </div>
        <hr class="card-divider" />
        <div class="links">
          <a
            v-if="data.issues_url"
            :href="data.issues_url"
            class="title"
            rel="noopener nofollow ugc external"
          >
            <IssuesIcon aria-hidden="true" />
            <span>Issues</span>
          </a>
          <a
            v-if="data.source_url"
            :href="data.source_url"
            class="title"
            rel="noopener nofollow ugc external"
          >
            <CodeIcon aria-hidden="true" />
            <span>Source</span>
          </a>
          <a
            v-if="data.wiki_url"
            :href="data.wiki_url"
            class="title"
            rel="noopener nofollow ugc external"
          >
            <WikiIcon aria-hidden="true" />
            <span>Wiki</span>
          </a>
          <a
            v-if="data.discord_url"
            :href="data.discord_url"
            class="title"
            rel="noopener nofollow ugc external"
          >
            <DiscordIcon aria-hidden="true" />
            <span>Discord</span>
          </a>
          <a
            v-for="(donation, index) in data.donation_urls"
            :key="index"
            :href="donation.url"
            rel="noopener nofollow ugc external"
          >
            <BuyMeACoffeeIcon v-if="donation.id === 'bmac'" aria-hidden="true" />
            <PatreonIcon v-else-if="donation.id === 'patreon'" aria-hidden="true" />
            <KoFiIcon v-else-if="donation.id === 'ko-fi'" aria-hidden="true" />
            <PaypalIcon v-else-if="donation.id === 'paypal'" aria-hidden="true" />
            <OpenCollectiveIcon v-else-if="donation.id === 'open-collective'" aria-hidden="true" />
            <HeartIcon v-else-if="donation.id === 'github'" />
            <CoinsIcon v-else />
            <span v-if="donation.id === 'bmac'">Buy Me a Coffee</span>
            <span v-else-if="donation.id === 'patreon'">Patreon</span>
            <span v-else-if="donation.id === 'paypal'">PayPal</span>
            <span v-else-if="donation.id === 'ko-fi'">Ko-fi</span>
            <span v-else-if="donation.id === 'github'">GitHub Sponsors</span>
            <span v-else>Donate</span>
          </a>
        </div>
      </Card>
    </div>
    <div v-if="data" class="content-container">
      <Promotion :external="false" query-param="?r=launcher" />
      <Card class="tabs">
        <NavRow
          v-if="data.gallery.length > 0"
          :links="[
            {
              label: 'Description',
              href: `/project/${$route.params.id}/`,
            },
            {
              label: 'Versions',
              href: `/project/${$route.params.id}/versions`,
            },
            {
              label: 'Gallery',
              href: `/project/${$route.params.id}/gallery`,
            },
          ]"
        />
        <NavRow
          v-else
          :links="[
            {
              label: 'Description',
              href: `/project/${$route.params.id}/`,
            },
            {
              label: 'Versions',
              href: `/project/${$route.params.id}/versions`,
            },
          ]"
        />
      </Card>
      <RouterView
        :project="data"
        :versions="versions"
        :members="members"
        :dependencies="dependencies"
        :instance="instance"
        :install="install"
        :installed="installed"
        :installing="installing"
        :installed-version="installedVersion"
      />
    </div>
  </div>
  <InstallConfirmModal ref="confirmModal" />
  <ModInstallModal ref="modInstallModal" />
  <IncompatibilityWarningModal ref="incompatibilityWarning" />
  <ContextMenu ref="options" @option-clicked="handleOptionsClick">
    <template #install> <DownloadIcon /> Install </template>
    <template #open_link> <GlobeIcon /> Open in Modrinth <ExternalIcon /> </template>
    <template #copy_link> <ClipboardCopyIcon /> Copy link </template>
  </ContextMenu>
</template>

<script setup>
import {
  Card,
  Avatar,
  Button,
  DownloadIcon,
  ReportIcon,
  HeartIcon,
  Categories,
  EnvironmentIndicator,
  UpdatedIcon,
  CalendarIcon,
  IssuesIcon,
  WikiIcon,
  Promotion,
  NavRow,
  CoinsIcon,
  CodeIcon,
  formatNumber,
  ExternalIcon,
  CheckIcon,
  GlobeIcon,
  ClipboardCopyIcon,
} from 'omorphia'
import {
  BuyMeACoffeeIcon,
  DiscordIcon,
  PatreonIcon,
  PaypalIcon,
  KoFiIcon,
  OpenCollectiveIcon,
} from '@/assets/external'
import { get_categories } from '@/helpers/tags'
import { install as packInstall } from '@/helpers/pack'
import {
  list,
  add_project_from_version as installMod,
  check_installed,
  get as getInstance,
  remove_project,
} from '@/helpers/profile'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useRoute } from 'vue-router'
import { ref, shallowRef, watch } from 'vue'
import { installVersionDependencies, isOffline } from '@/helpers/utils'
import InstallConfirmModal from '@/components/ui/InstallConfirmModal.vue'
import ModInstallModal from '@/components/ui/ModInstallModal.vue'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import IncompatibilityWarningModal from '@/components/ui/IncompatibilityWarningModal.vue'
import { useFetch } from '@/helpers/fetch.js'
import { handleError } from '@/store/notifications.js'
import { convertFileSrc } from '@tauri-apps/api/tauri'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import { mixpanel_track } from '@/helpers/mixpanel'

const route = useRoute()
const breadcrumbs = useBreadcrumbs()

const confirmModal = ref(null)
const modInstallModal = ref(null)
const incompatibilityWarning = ref(null)

const options = ref(null)
const installing = ref(false)
const data = shallowRef(null)
const versions = shallowRef([])
const members = shallowRef([])
const dependencies = shallowRef([])
const categories = shallowRef([])
const instance = ref(null)

const installed = ref(false)
const installedVersion = ref(null)

const offline = ref(await isOffline())

async function fetchProjectData() {
  ;[
    data.value,
    versions.value,
    members.value,
    dependencies.value,
    categories.value,
    instance.value,
  ] = await Promise.all([
    useFetch(`https://staging-api.modrinth.com/v2/project/${route.params.id}`, 'project'),
    useFetch(`https://staging-api.modrinth.com/v2/project/${route.params.id}/version`, 'project'),
    useFetch(`https://staging-api.modrinth.com/v2/project/${route.params.id}/members`, 'project'),
    useFetch(
      `https://staging-api.modrinth.com/v2/project/${route.params.id}/dependencies`,
      'project'
    ),
    get_categories().catch(handleError),
    route.query.i ? getInstance(route.query.i, false).catch(handleError) : Promise.resolve(),
  ])

  installed.value =
    instance.value?.path &&
    (await check_installed(instance.value.path, data.value.id).catch(handleError))
  breadcrumbs.setName('Project', data.value.title)
  installedVersion.value = instance.value
    ? Object.values(instance.value.projects).find(
        (p) => p?.metadata?.version?.project_id === data.value.id
      )?.metadata?.version?.id
    : null
}

if (!offline.value) await fetchProjectData()

watch(
  () => route.params.id,
  async () => {
    if (route.params.id && route.path.startsWith('/project')) {
      await fetchProjectData()
    }
  }
)

dayjs.extend(relativeTime)

const markInstalled = () => {
  installed.value = true
}

async function install(version) {
  installing.value = true
  let queuedVersionData
  if (instance.value) {
    instance.value = await getInstance(instance.value.path, false).catch(handleError)
  }

  if (installed.value) {
    const old_project = Object.entries(instance.value.projects)
      .map(([key, value]) => ({
        key,
        value,
      }))
      .find((p) => p.value.metadata?.version?.project_id === data.value.id)
    if (!old_project) {
      // Switching too fast, old project is not recognized as a Modrinth project yet
      installing.value = false
      return
    }

    await remove_project(instance.value.path, old_project.key)
  }

  if (version) {
    queuedVersionData = versions.value.find((v) => v.id === version)
  } else {
    if (data.value.project_type === 'modpack' || !instance.value) {
      queuedVersionData = versions.value[0]
    } else {
      queuedVersionData = versions.value.find((v) =>
        v.game_versions.includes(data.value.game_versions[0])
      )
    }
  }

  if (data.value.project_type === 'modpack') {
    const packs = Object.values(await list(true).catch(handleError))
    if (
      packs.length === 0 ||
      !packs
        .map((value) => value.metadata)
        .find((pack) => pack.linked_data?.modrinth_modpack?.project_id === data.value.id)
    ) {
      await packInstall(
        data.value.id,
        queuedVersionData.id,
        data.value.title,
        data.value.icon_url
      ).catch(handleError)

      mixpanel_track('PackInstall', {
        id: data.value.id,
        version_id: queuedVersionData.id,
        title: data.value.title,
        source: 'ProjectPage',
      })
    } else {
      confirmModal.value.show(
        data.value.id,
        queuedVersionData.id,
        data.value.title,
        data.value.icon_url
      )
    }
  } else {
    if (instance.value) {
      if (!version) {
        const gameVersion = instance.value.metadata.game_version
        const loader = instance.value.metadata.loader
        const selectedVersion = versions.value.find(
          (v) =>
            v.game_versions.includes(gameVersion) &&
            (data.value.project_type === 'mod'
              ? v.loaders.includes(loader) || v.loaders.includes('minecraft')
              : true)
        )
        if (!selectedVersion) {
          incompatibilityWarning.value.show(
            instance.value,
            data.value.title,
            versions.value,
            markInstalled,
            data.value.id,
            data.value.project_type
          )
          installing.value = false
          return
        } else {
          queuedVersionData = selectedVersion
          await installMod(instance.value.path, selectedVersion.id).catch(handleError)
          await installVersionDependencies(instance.value, queuedVersionData)
          installedVersion.value = selectedVersion.id
          mixpanel_track('ProjectInstall', {
            loader: instance.value.metadata.loader,
            game_version: instance.value.metadata.game_version,
            id: data.value.id,
            project_type: data.value.project_type,
            version_id: queuedVersionData.id,
            title: data.value.title,
            source: 'ProjectPage',
          })
        }
      } else {
        const gameVersion = instance.value.metadata.game_version
        const loader = instance.value.metadata.loader
        const compatible = versions.value.some(
          (v) =>
            v.game_versions.includes(gameVersion) &&
            (data.value.project_type === 'mod'
              ? v.loaders.includes(loader) || v.loaders.includes('minecraft')
              : true)
        )
        if (compatible) {
          await installMod(instance.value.path, queuedVersionData.id).catch(handleError)
          await installVersionDependencies(instance.value, queuedVersionData)
          installedVersion.value = queuedVersionData.id
          mixpanel_track('ProjectInstall', {
            loader: instance.value.metadata.loader,
            game_version: instance.value.metadata.game_version,
            id: data.value.id,
            project_type: data.value.project_type,
            version_id: queuedVersionData.id,
            title: data.value.title,
            source: 'ProjectPage',
          })
        } else {
          incompatibilityWarning.value.show(
            instance.value,
            data.value.title,
            [queuedVersionData],
            markInstalled,
            data.value.id,
            data.value.project_type
          )
          installing.value = false
          return
        }
      }
      installed.value = true
    } else {
      modInstallModal.value.show(
        data.value.id,
        version ? [versions.value.find((v) => v.id === queuedVersionData.id)] : versions.value,
        data.value.title,
        data.value.project_type
      )
    }
  }

  installing.value = false
}

const handleRightClick = (e) => {
  options.value.showMenu(e, data.value, [
    { name: 'install' },
    { type: 'divider' },
    { name: 'open_link' },
    { name: 'copy_link' },
  ])
}

const handleOptionsClick = (args) => {
  switch (args.option) {
    case 'install':
      install(null)
      break
    case 'open_link':
      window.__TAURI_INVOKE__('tauri', {
        __tauriModule: 'Shell',
        message: {
          cmd: 'open',
          path: `https://modrinth.com/${args.item.project_type}/${args.item.slug}`,
        },
      })
      break
    case 'copy_link':
      navigator.clipboard.writeText(
        `https://modrinth.com/${args.item.project_type}/${args.item.slug}`
      )
      break
  }
}
</script>

<style scoped lang="scss">
.root-container {
  display: flex;
  flex-direction: row;
  min-height: 100%;
}

.project-sidebar {
  position: fixed;
  width: 20rem;
  min-height: calc(100vh - 3.25rem);
  height: fit-content;
  max-height: calc(100vh - 3.25rem);
  padding: 1rem 0.5rem 1rem 1rem;
  overflow-y: auto;
  -ms-overflow-style: none;
  scrollbar-width: none;

  &::-webkit-scrollbar {
    width: 0;
    background: transparent;
  }
}

.sidebar-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.content-container {
  display: flex;
  flex-direction: column;
  width: 100%;
  padding: 1rem;
  margin-left: 19.5rem;
}

.button-group {
  display: flex;
  flex-wrap: wrap;
  flex-direction: row;
  gap: 0.5rem;
}

.stats {
  display: flex;
  flex-direction: column;
  flex-wrap: wrap;
  gap: var(--gap-md);

  .stat {
    display: flex;
    flex-direction: row;
    align-items: center;
    width: fit-content;
    gap: var(--gap-xs);
    --stat-strong-size: 1.25rem;

    strong {
      font-size: var(--stat-strong-size);
    }

    p {
      margin: 0;
    }

    svg {
      min-height: var(--stat-strong-size);
      min-width: var(--stat-strong-size);
    }
  }

  .date {
    margin-top: auto;
  }
}

.tabs {
  display: flex;
  flex-direction: row;
  gap: 1rem;
  margin-bottom: var(--gap-md);
  justify-content: space-between;

  .tab {
    display: flex;
    flex-direction: row;
    align-items: center;
    border-radius: var(--border-radius);
    cursor: pointer;
    transition: background-color 0.2s ease-in-out;

    &:hover {
      background-color: var(--color-raised-bg);
    }

    &.router-view-active {
      background-color: var(--color-raised-bg);
    }
  }
}

.links {
  a {
    display: inline-flex;
    align-items: center;
    border-radius: 1rem;
    color: var(--color-text);

    svg,
    img {
      height: 1rem;
      width: 1rem;
    }

    span {
      margin-left: 0.25rem;
      text-decoration: underline;
      line-height: 2rem;
    }

    &:focus-visible,
    &:hover {
      svg,
      img,
      span {
        color: var(--color-heading);
      }
    }

    &:active {
      svg,
      img,
      span {
        color: var(--color-text-dark);
      }
    }

    &:not(:last-child)::after {
      content: '•';
      margin: 0 0.25rem;
    }
  }
}

.install-loading {
  scale: 0.2;
  height: 1rem;
  width: 1rem;
  margin-right: -1rem;

  :deep(svg) {
    color: var(--color-contrast);
  }
}

.small-instance {
  padding: var(--gap-lg);
  border-radius: var(--radius-md);
  margin-bottom: var(--gap-md);

  .instance {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0;

    .title {
      font-weight: 600;
      color: var(--color-contrast);
    }
  }

  .small-instance_info {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0.25rem 0;
  }
}
</style>
