<script setup>
import { Modal, Button } from 'omorphia'
import { ref } from 'vue'
import { handleError } from '@/store/notifications.js'
import { share_accept, share_install, share_get_link_id } from '@/helpers/shared_profiles.js'

const confirmModal = ref(null)
const linkId = ref(null)
const sharedProfile = ref(null)

defineExpose({
  async show(event) {
    console.log('showing accept shared profile modal', event)
    linkId.value = event.link
    sharedProfile.value = await share_get_link_id(linkId.value).catch(handleError)
    confirmModal.value.show()
    console.log('sharedProfile')
  },
})

async function install() {
  confirmModal.value.hide()
  await share_accept(linkId.value).catch(handleError)
  await share_install(sharedProfile.value.id).catch(handleError)
}
</script>

<template>
  <Modal ref="confirmModal" :header="`Install ${sharedProfile?.name}`">
    <div class="modal-body">
      <div class="button-row">
        <div class="markdown-body">
          <p>
            Installing <code>{{ sharedProfile.name }}</code> from user {{ sharedProfile.owner_id }}
          </p>
        </div>
        <div class="button-group">
          <Button color="primary" @click="install">Install</Button>
        </div>
      </div>
    </div>
  </Modal>
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
