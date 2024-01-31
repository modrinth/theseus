<script setup>
import { Button, Modal, CheckIcon, Badge } from 'omorphia'
import { computed, ref } from 'vue'
import { useTheming } from '@/store/theme'
import { update_managed_modrinth_version } from '@/helpers/profile'
import { releaseColor } from '@/helpers/utils'
import { SwapIcon } from '@/assets/icons/index.js'

const props = defineProps({
  versions: {
    type: Array,
    required: true,
  },
  instance: {
    type: Object,
    default: null,
  },
})

defineExpose({
  show: () => {
    modpackVersionModal.value.show()
  },
})

const filteredVersions = computed(() => {
  return props.versions
})

const modpackVersionModal = ref(null)
const installedVersion = computed(
  () => props.instance?.metadata?.linked_data?.modrinth_modpack?.version_id
)
const installing = computed(() => props.instance.install_stage !== 'installed')
const inProgress = ref(false)

const themeStore = useTheming()

const switchVersion = async (versionId) => {
  inProgress.value = true
  await update_managed_modrinth_version(props.instance.path, versionId)
  inProgress.value = false
}
</script>

<template>
  <Modal
    ref="modpackVersionModal"
    class="modpack-version-modal"
    header="Change modpack version"
    :noblur="!themeStore.advancedRendering"
  >
    <div class="modal-body">
      <Card v-if="instance.metadata.linked_data?.modrinth_modpack" class="mod-card">
        <div class="table">
          <div class="table-row with-columns table-head">
            <div class="table-cell table-text download-cell" />
            <div class="name-cell table-cell table-text">Name</div>
            <div class="table-cell table-text">Supports</div>
          </div>
          <div class="scrollable">
            <div
              v-for="version in filteredVersions"
              :key="version.id"
              class="table-row with-columns selectable"
              @click="$router.push(`/project/${$route.params.id}/version/${version.id}`)"
            >
              <div class="table-cell table-text">
                <Button
                  :color="version.id === installedVersion ? '' : 'primary'"
                  icon-only
                  :disabled="inProgress || installing || version.id === installedVersion"
                  @click.stop="() => switchVersion(version.id)"
                >
                  <SwapIcon v-if="version.id !== installedVersion" />
                  <CheckIcon v-else />
                </Button>
              </div>
              <div class="name-cell table-cell table-text">
                <div class="version-link">
                  {{ version.name.charAt(0).toUpperCase() + version.name.slice(1) }}
                  <div class="version-badge">
                    <div class="channel-indicator">
                      <Badge
                        :color="releaseColor(version.version_type)"
                        :type="
                          version.version_type.charAt(0).toUpperCase() +
                          version.version_type.slice(1)
                        "
                      />
                    </div>
                    <div>
                      {{ version.version_number }}
                    </div>
                  </div>
                </div>
              </div>
              <div class="table-cell table-text stacked-text">
                <span>
                  {{
                    version.loaders
                      .map((str) => str.charAt(0).toUpperCase() + str.slice(1))
                      .join(', ')
                  }}
                </span>
                <span>
                  {{ version.game_versions.join(', ') }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  </Modal>
</template>

<style scoped lang="scss">
.filter-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.with-columns {
  grid-template-columns: min-content 1fr 1fr;
}

.scrollable {
  overflow-y: auto;
  max-height: 25rem;
}

.card-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background-color: var(--color-raised-bg);
}

.mod-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  overflow: hidden;
  margin-top: 0.5rem;
}

.version-link {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  .version-badge {
    display: flex;
    flex-wrap: wrap;

    .channel-indicator {
      margin-right: 0.5rem;
    }
  }
}

.stacked-text {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  align-items: flex-start;
}

.download-cell {
  width: 4rem;
  padding: 1rem;
}

.modal-body {
  padding: var(--gap-xl);
  display: flex;
  flex-direction: column;
  gap: var(--gap-md);
}

.table {
  border: 1px solid var(--color-bg);
}
</style>
