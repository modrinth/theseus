<script setup>
import { Modal, Button } from 'omorphia'
import { ref } from 'vue'
import { useFetch } from '@/helpers/fetch.js'
import SearchCard from '@/components/ui/SearchCard.vue'
import { get_categories } from '@/helpers/tags.js'
import { handleError } from '@/store/notifications.js'
import { install as packInstall } from '@/helpers/pack.js'
import mixpanel from 'mixpanel-browser'
import ModInstallModal from '@/components/ui/ModInstallModal.vue'

const confirmModal = ref(null)
const project = ref(null)
const versions = ref(null)
const categories = ref(null)
const installing = ref(false)
const modInstallModal = ref(null)

defineExpose({
  async show(event) {
    if (event.event === 'InstallVersion') {
      versions.value[0] = await useFetch(
        `https://api.modrinth.com/v2/version/${encodeURIComponent(event.id)}`,
        'version',
      )
      project.value = await useFetch(
        `https://api.modrinth.com/v2/project/${encodeURIComponent(versions.value[0].project_id)}`,
        'project',
      )
    } else {
      project.value = await useFetch(
        `https://api.modrinth.com/v2/project/${encodeURIComponent(event.id)}`,
        'project',
      )
      versions.value = await useFetch(
        `https://api.modrinth.com/v2/project/${encodeURIComponent(event.id)}/version`,
        'project versions',
      )
    }
    categories.value = (await get_categories().catch(handleError)).filter(
      (cat) => project.value.categories.includes(cat.name) && cat.project_type === 'mod',
    )
    confirmModal.value.show()
    categories.value = (await get_categories().catch(handleError)).filter(
      (cat) => project.value.categories.includes(cat.name) && cat.project_type === 'mod',
    )
    confirmModal.value.show()
  },
})

async function install() {
  confirmModal.value.hide()
  if (project.value.project_type === 'modpack') {
    await packInstall(
      project.value.id,
      versions.value[0].id,
      project.value.title,
      project.value.icon_url,
    ).catch(handleError)

    mixpanel.track('PackInstall', {
      id: project.value.id,
      version_id: versions.value[0].id,
      title: project.value.title,
      source: 'ProjectPage',
    })
  } else {
    modInstallModal.value.show(
      project.value.id,
      versions.value,
      project.value.title,
      project.value.project_type,
    )
  }
}
</script>

<template>
  <Modal ref="confirmModal" :header="`Install ${project?.title}`">
    <div class="modal-body">
      <SearchCard
        :project="project"
        class="project-card"
        :categories="categories"
        @open="confirmModal.hide()"
      />
      <div class="button-row">
        <div class="markdown-body">
          <p>
            Installing <code>{{ project.id }}</code> from Modrinth
          </p>
        </div>
        <div class="button-group">
          <Button :loading="installing" color="primary" @click="install">Install</Button>
        </div>
      </div>
    </div>
  </Modal>
  <ModInstallModal ref="modInstallModal" />
</template>

<style scoped lang="scss">
.modal-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--gap-md);
  padding: var(--gap-lg);
}

.button-row {
  width: 100%;
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  gap: var(--gap-md);
}

.button-group {
  display: flex;
  flex-direction: row;
  gap: var(--gap-sm);
}

.project-card {
  background-color: var(--color-bg);
  width: 100%;

  :deep(.badge) {
    border: 1px solid var(--color-raised-bg);
    background-color: var(--color-accent-contrast);
  }
}
</style>
