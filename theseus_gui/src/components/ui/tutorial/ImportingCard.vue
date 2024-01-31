<script setup>
import {
  Button,
  Card,
  Checkbox,
  Chips,
  XIcon,
  FolderOpenIcon,
  FolderSearchIcon,
  UpdatedIcon,
} from 'omorphia'
import { ref } from 'vue'
import {
  get_default_launcher_path,
  get_importable_instances,
  import_instance,
} from '@/helpers/import.js'
import { open } from '@tauri-apps/api/dialog'
import { handleError } from '@/store/state.js'

const props = defineProps({
  nextPage: {
    type: Function,
    required: true,
  },
  prevPage: {
    type: Function,
    required: true,
  },
})

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
Promise.all(promises)

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
  profiles.value.set(
    selectedProfileType.value.name,
    instances.map((name) => ({ name, selected: false }))
  )
}

const setPath = () => {
  profileOptions.value.find((profile) => profile.name === selectedProfileType.value.name).path =
    selectedProfileType.value.path
}

const next = async () => {
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
    }
  }
  loading.value = false
  props.nextPage()
}
</script>

<template>
  <Card>
    <h2>Importing external profiles</h2>
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
            :model-value="profiles.get(selectedProfileType.name)?.every((child) => child.selected)"
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
      <Button class="transparent" @click="prevPage"> Back </Button>
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
      <Button class="transparent" @click="nextPage"> Next </Button>
    </div>
  </Card>
</template>

<style scoped lang="scss">
.card {
  padding: var(--gap-xl);
  min-height: unset;
  overflow-y: auto;
}

.path-selection {
  padding: var(--gap-xl);
  background-color: var(--color-bg);
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
  margin-bottom: var(--gap-md);
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
  margin-bottom: var(--gap-md);
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
</style>
