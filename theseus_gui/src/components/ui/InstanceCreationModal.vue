<template>
  <Modal ref="modal" header="Create instance" :noblur="!themeStore.advancedRendering">
    <div class="modal-header">
      <Chips v-model="creationType" :items="['custom', 'from file', 'import from launcher']" />
    </div>
    <hr class="card-divider" />
    <div v-if="creationType === 'custom'" class="modal-body">
      <div class="image-upload">
        <Avatar :src="display_icon" size="md" :rounded="true" />
        <div class="image-input">
          <Button @click="upload_icon()">
            <UploadIcon />
            Select icon
          </Button>
          <Button :disabled="!display_icon" @click="reset_icon">
            <XIcon />
            Remove icon
          </Button>
        </div>
      </div>
      <div class="input-row">
        <p class="input-label">Name</p>
        <input
          v-model="profile_name"
          autocomplete="off"
          class="text-input"
          type="text"
          maxlength="100"
        />
      </div>
      <div class="input-row">
        <p class="input-label">Loader</p>
        <Chips v-model="loader" :items="loaders" />
      </div>
      <div class="input-row">
        <p class="input-label">Game version</p>
        <div class="versions">
          <multiselect
            v-model="game_version"
            class="selector"
            :options="game_versions"
            :multiple="false"
            :searchable="true"
            placeholder="Select game version"
            open-direction="top"
            :show-labels="false"
          />
          <Checkbox
            v-if="showAdvanced"
            v-model="showSnapshots"
            class="filter-checkbox"
            label="Include snapshots"
          />
        </div>
      </div>
      <div v-if="showAdvanced && loader !== 'vanilla'" class="input-row">
        <p class="input-label">Loader version</p>
        <Chips v-model="loader_version" :items="['stable', 'latest', 'other']" />
      </div>
      <div v-if="showAdvanced && loader_version === 'other' && loader !== 'vanilla'">
        <div v-if="game_version" class="input-row">
          <p class="input-label">Select version</p>
          <multiselect
            v-model="specified_loader_version"
            class="selector"
            :options="selectable_versions"
            :searchable="true"
            placeholder="Select loader version"
            open-direction="top"
            :show-labels="false"
          />
        </div>
        <div v-else class="input-row">
          <p class="warning">Select a game version before you select a loader version</p>
        </div>
      </div>
      <div class="input-group push-right">
        <Button @click="toggle_advanced">
          <CodeIcon />
          {{ showAdvanced ? 'Hide advanced' : 'Show advanced' }}
        </Button>
        <Button @click="hide()">
          <XIcon />
          Cancel
        </Button>
        <Button color="primary" :disabled="!check_valid || creating" @click="create_instance()">
          <PlusIcon v-if="!creating" />
          {{ creating ? 'Creating...' : 'Create' }}
        </Button>
      </div>
    </div>
    <div v-else-if="creationType === 'from file'" class="modal-body">
      <Button @click="openFile"> <FolderOpenIcon /> Import from file </Button>
      <div class="info"><InfoIcon /> Or drag and drop your .mrpack file</div>
    </div>
    <div v-else class="modal-body">
      <Chips
        v-model="selectedProfileType"
        :items="profileOptions"
        :format-label="(profile) => profile?.name"
      />
      <div class="path-selection">
        <h3>{{ selectedProfileType.name }} path</h3>
        <div class="path-input">
          <div class="iconified-input">
            <FolderOpenIcon />
            <input
              v-model="selectedProfileType.path"
              type="text"
              placeholder="Path to launcher"
              @change="setPath"
            />
            <Button class="r-btn" @click="() => (selectedLauncherPath = '')">
              <XIcon />
            </Button>
          </div>
          <Button icon-only @click="selectLauncherPath">
            <FolderSearchIcon />
          </Button>
          <Button icon-only @click="reload">
            <UpdatedIcon />
          </Button>
        </div>
      </div>
      <div class="table">
        <div class="table-head table-row">
          <div class="toggle-all table-cell">
            <Checkbox
              class="select-checkbox"
              :model-value="
                profiles.get(selectedProfileType.name)?.every((child) => child.selected)
              "
              @update:model-value="
                (newValue) =>
                  profiles
                    .get(selectedProfileType.name)
                    ?.forEach((child) => (child.selected = newValue))
              "
            />
          </div>
          <div class="name-cell table-cell">Profile name</div>
        </div>
        <div
          v-if="
            profiles.get(selectedProfileType.name) &&
            profiles.get(selectedProfileType.name).length > 0
          "
          class="table-content"
        >
          <div
            v-for="(profile, index) in profiles.get(selectedProfileType.name)"
            :key="index"
            class="table-row"
          >
            <div class="checkbox-cell table-cell">
              <Checkbox v-model="profile.selected" class="select-checkbox" />
            </div>
            <div class="name-cell table-cell">
              {{ profile.name }}
            </div>
          </div>
        </div>
        <div v-else class="table-content empty">No profiles found</div>
      </div>
      <div class="button-row">
        <Button
          :disabled="
            loading ||
            !Array.from(profiles.values())
              .flatMap((e) => e)
              .some((e) => e.selected)
          "
          color="primary"
          @click="next"
        >
          {{
            loading
              ? 'Importing...'
              : Array.from(profiles.values())
                  .flatMap((e) => e)
                  .some((e) => e.selected)
              ? `Import ${
                  Array.from(profiles.values())
                    .flatMap((e) => e)
                    .filter((e) => e.selected).length
                } profiles`
              : 'Select profiles to import'
          }}
        </Button>
        <ProgressBar
          v-if="loading"
          :progress="(importedProfiles / (totalProfiles + 0.0001)) * 100"
        />
      </div>
    </div>
  </Modal>
</template>

<script setup>
import {
  Avatar,
  Button,
  Chips,
  Modal,
  PlusIcon,
  UploadIcon,
  XIcon,
  CodeIcon,
  Checkbox,
  FolderOpenIcon,
  InfoIcon,
  FolderSearchIcon,
  UpdatedIcon,
} from 'omorphia'
import { computed, onUnmounted, ref, shallowRef } from 'vue'
import { get_loaders } from '@/helpers/tags'
import { create } from '@/helpers/profile'
import { open } from '@tauri-apps/api/dialog'
import { tauri } from '@tauri-apps/api'
import {
  get_game_versions,
  get_fabric_versions,
  get_forge_versions,
  get_quilt_versions,
  get_neoforge_versions,
} from '@/helpers/metadata'
import { handleError } from '@/store/notifications.js'
import Multiselect from 'vue-multiselect'
import { mixpanel_track } from '@/helpers/mixpanel'
import { useTheming } from '@/store/state.js'
import { listen } from '@tauri-apps/api/event'
import { install_from_file } from '@/helpers/pack.js'
import {
  get_default_launcher_path,
  get_importable_instances,
  import_instance,
} from '@/helpers/import.js'
import ProgressBar from '@/components/ui/ProgressBar.vue'

const themeStore = useTheming()

const profile_name = ref('')
const game_version = ref('')
const loader = ref('vanilla')
const loader_version = ref('stable')
const specified_loader_version = ref('')
const icon = ref(null)
const display_icon = ref(null)
const showAdvanced = ref(false)
const creating = ref(false)
const showSnapshots = ref(false)
const creationType = ref('from file')
const isShowing = ref(false)

defineExpose({
  show: async () => {
    game_version.value = ''
    specified_loader_version.value = ''
    profile_name.value = ''
    creating.value = false
    showAdvanced.value = false
    showSnapshots.value = false
    loader.value = 'vanilla'
    loader_version.value = 'stable'
    icon.value = null
    display_icon.value = null
    isShowing.value = true
    modal.value.show()

    unlistener.value = await listen('tauri://file-drop', async (event) => {
      // Only if modal is showing
      if (!isShowing.value) return
      if (creationType.value !== 'from file') return
      hide()
      if (event.payload && event.payload.length > 0 && event.payload[0].endsWith('.mrpack')) {
        await install_from_file(event.payload[0]).catch(handleError)
        mixpanel_track('InstanceCreate', {
          source: 'CreationModalFileDrop',
        })
      }
    })

    mixpanel_track('InstanceCreateStart', { source: 'CreationModal' })
  },
})

const unlistener = ref(null)
const hide = () => {
  isShowing.value = false
  modal.value.hide()
  if (unlistener.value) {
    unlistener.value()
    unlistener.value = null
  }
}
onUnmounted(() => {
  if (unlistener.value) {
    unlistener.value()
    unlistener.value = null
  }
})

const [
  fabric_versions,
  forge_versions,
  quilt_versions,
  neoforge_versions,
  all_game_versions,
  loaders,
] = await Promise.all([
  get_fabric_versions().then(shallowRef).catch(handleError),
  get_forge_versions().then(shallowRef).catch(handleError),
  get_quilt_versions().then(shallowRef).catch(handleError),
  get_neoforge_versions().then(shallowRef).catch(handleError),
  get_game_versions().then(shallowRef).catch(handleError),
  get_loaders()
    .then((value) =>
      value
        .filter((item) => item.supported_project_types.includes('modpack'))
        .map((item) => item.name.toLowerCase())
    )
    .then(ref)
    .catch(handleError),
])
loaders.value.unshift('vanilla')

const game_versions = computed(() => {
  return all_game_versions.value.versions
    .filter((item) => {
      let defaultVal = item.type === 'release' || showSnapshots.value
      if (loader.value === 'fabric') {
        defaultVal &= fabric_versions.value.gameVersions.some((x) => item.id === x.id)
      } else if (loader.value === 'forge') {
        defaultVal &= forge_versions.value.gameVersions.some((x) => item.id === x.id)
      } else if (loader.value === 'quilt') {
        defaultVal &= quilt_versions.value.gameVersions.some((x) => item.id === x.id)
      } else if (loader.value === 'neoforge') {
        defaultVal &= neoforge_versions.value.gameVersions.some((x) => item.id === x.id)
      }

      return defaultVal
    })
    .map((item) => item.id)
})

const modal = ref(null)

const check_valid = computed(() => {
  return (
    profile_name.value.trim() &&
    game_version.value &&
    game_versions.value.includes(game_version.value)
  )
})

const create_instance = async () => {
  creating.value = true
  const loader_version_value =
    loader_version.value === 'other' ? specified_loader_version.value : loader_version.value
  const loaderVersion = loader.value === 'vanilla' ? null : loader_version_value ?? 'stable'

  hide()
  creating.value = false

  await create(
    profile_name.value,
    game_version.value,
    loader.value,
    loader.value === 'vanilla' ? null : loader_version_value ?? 'stable',
    icon.value
  ).catch(handleError)

  mixpanel_track('InstanceCreate', {
    profile_name: profile_name.value,
    game_version: game_version.value,
    loader: loader.value,
    loader_version: loaderVersion,
    has_icon: !!icon.value,
    source: 'CreationModal',
  })
}

const upload_icon = async () => {
  icon.value = await open({
    multiple: false,
    filters: [
      {
        name: 'Image',
        extensions: ['png', 'jpeg', 'svg', 'webp', 'gif', 'jpg'],
      },
    ],
  })

  if (!icon.value) return
  display_icon.value = tauri.convertFileSrc(icon.value)
}

const reset_icon = () => {
  icon.value = null
  display_icon.value = null
}

const selectable_versions = computed(() => {
  if (game_version.value) {
    if (loader.value === 'fabric') {
      return fabric_versions.value.gameVersions[0].loaders.map((item) => item.id)
    } else if (loader.value === 'forge') {
      return forge_versions.value.gameVersions
        .find((item) => item.id === game_version.value)
        .loaders.map((item) => item.id)
    } else if (loader.value === 'quilt') {
      return quilt_versions.value.gameVersions[0].loaders.map((item) => item.id)
    } else if (loader.value === 'neoforge') {
      return neoforge_versions.value.gameVersions
        .find((item) => item.id === game_version.value)
        .loaders.map((item) => item.id)
    }
  }
  return []
})

const toggle_advanced = () => {
  showAdvanced.value = !showAdvanced.value
}

const openFile = async () => {
  const newProject = await open({ multiple: false })
  if (!newProject) return
  hide()
  await install_from_file(newProject).catch(handleError)

  mixpanel_track('InstanceCreate', {
    source: 'CreationModalFileOpen',
  })
}

const profiles = ref(
  new Map([
    ['MultiMC', []],
    ['GDLauncher', []],
    ['ATLauncher', []],
    ['Curseforge', []],
    ['PrismLauncher', []],
  ])
)

const loading = ref(false)
const importedProfiles = ref(0)
const totalProfiles = ref(0)

const selectedProfileType = ref('MultiMC')
const profileOptions = ref([
  { name: 'MultiMC', path: '' },
  { name: 'GDLauncher', path: '' },
  { name: 'ATLauncher', path: '' },
  { name: 'Curseforge', path: '' },
  { name: 'PrismLauncher', path: '' },
])

// Attempt to get import profiles on default paths
const promises = profileOptions.value.map(async (option) => {
  const path = await get_default_launcher_path(option.name).catch(handleError)
  if (!path || path === '') return

  // Try catch to allow failure and simply ignore default path attempt
  try {
    const instances = await get_importable_instances(option.name, path)

    if (!instances) return
    profileOptions.value.find((profile) => profile.name === option.name).path = path
    profiles.value.set(
      option.name,
      instances.map((name) => ({ name, selected: false }))
    )
  } catch (error) {
    // Allow failure silently
  }
})
await Promise.all(promises)

const selectLauncherPath = async () => {
  selectedProfileType.value.path = await open({ multiple: false, directory: true })

  if (selectedProfileType.value.path) {
    await reload()
  }
}

const reload = async () => {
  const instances = await get_importable_instances(
    selectedProfileType.value.name,
    selectedProfileType.value.path
  ).catch(handleError)
  if (instances) {
    profiles.value.set(
      selectedProfileType.value.name,
      instances.map((name) => ({ name, selected: false }))
    )
  } else {
    profiles.value.set(selectedProfileType.value.name, [])
  }
}

const setPath = () => {
  profileOptions.value.find((profile) => profile.name === selectedProfileType.value.name).path =
    selectedProfileType.value.path
}

const next = async () => {
  importedProfiles.value = 0
  totalProfiles.value = Array.from(profiles.value.values())
    .map((profiles) => profiles.filter((profile) => profile.selected).length)
    .reduce((a, b) => a + b, 0)
  loading.value = true
  for (const launcher of Array.from(profiles.value.entries()).map(([launcher, profiles]) => ({
    launcher,
    path: profileOptions.value.find((option) => option.name === launcher).path,
    profiles,
  }))) {
    for (const profile of launcher.profiles.filter((profile) => profile.selected)) {
      await import_instance(launcher.launcher, launcher.path, profile.name)
        .catch(handleError)
        .then(() => console.log(`Successfully Imported ${profile.name} from ${launcher.launcher}`))
      profile.selected = false
      importedProfiles.value++
    }
  }
  loading.value = false
}
</script>

<style lang="scss" scoped>
.modal-body {
  display: flex;
  flex-direction: column;
  padding: var(--gap-lg);
  gap: var(--gap-md);
}

.input-label {
  font-size: 1rem;
  font-weight: bolder;
  color: var(--color-contrast);
  margin-bottom: 0.5rem;
}

.text-input {
  width: 20rem;
}

.image-upload {
  display: flex;
  gap: 1rem;
}

.image-input {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  justify-content: center;
}

.warning {
  font-style: italic;
}

.versions {
  display: flex;
  flex-direction: row;
  gap: 1rem;
}

:deep(button.checkbox) {
  border: none;
}

.selector {
  max-width: 20rem;
}

.labeled-divider {
  text-align: center;
}

.labeled-divider:after {
  background-color: var(--color-raised-bg);
  content: 'Or';
  color: var(--color-base);
  padding: var(--gap-sm);
  position: relative;
  top: -0.5rem;
}

.info {
  display: flex;
  flex-direction: row;
  gap: 0.5rem;
  align-items: center;
}

.modal-header {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  padding: var(--gap-lg);
  padding-bottom: 0;
}

.path-selection {
  padding: var(--gap-xl);
  background-color: var(--color-bg);
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
  gap: var(--gap-md);

  h3 {
    margin: 0;
  }

  .path-input {
    display: flex;
    align-items: center;
    width: 100%;
    flex-direction: row;
    gap: var(--gap-sm);

    .iconified-input {
      flex-grow: 1;
      :deep(input) {
        width: 100%;
        flex-basis: auto;
      }
    }
  }
}

.table {
  border: 1px solid var(--color-bg);
}

.table-row {
  grid-template-columns: min-content auto;
}

.table-content {
  max-height: calc(5 * (18px + 2rem));
  height: calc(5 * (18px + 2rem));
  overflow-y: auto;
}

.select-checkbox {
  button.checkbox {
    border: none;
  }
}

.button-row {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  gap: var(--gap-md);

  .transparent {
    padding: var(--gap-sm) 0;
  }
}

.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
  font-weight: bolder;
  color: var(--color-contrast);
}

.card-divider {
  margin: var(--gap-md) var(--gap-lg) 0 var(--gap-lg);
}
</style>
