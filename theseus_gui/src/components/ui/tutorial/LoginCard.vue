<script setup>
import { Button, LogInIcon, Modal, ClipboardCopyIcon, GlobeIcon, Card } from 'omorphia'
import { authenticate_await_completion, authenticate_begin_flow } from '@/helpers/auth.js'
import { handleError } from '@/store/notifications.js'
import { useTheming } from '@/store/theme.js'
import mixpanel from 'mixpanel-browser'
import { get, set } from '@/helpers/settings.js'
import { ref } from 'vue'
import QrcodeVue from 'qrcode.vue'

const themeStore = useTheming()
const loginUrl = ref(null)
const loginModal = ref()
const loginCode = ref(null)
const finalizedLogin = ref(false)

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

async function login() {
  const loginSuccess = await authenticate_begin_flow().catch(handleError)
  loginUrl.value = loginSuccess.verification_uri
  loginCode.value = loginSuccess.user_code
  loginModal.value.show()

  await window.__TAURI_INVOKE__('tauri', {
    __tauriModule: 'Shell',
    message: {
      cmd: 'open',
      path: loginSuccess.verification_uri,
    },
  })

  const loggedIn = await authenticate_await_completion().catch(handleError)
  loginModal.value.hide()

  const settings = await get().catch(handleError)
  settings.default_user = loggedIn.id
  await set(settings).catch(handleError)
  finalizedLogin.value = true
  await mixpanel.track('AccountLogIn')
  props.nextPage()
}

const openUrl = async () => {
  await window.__TAURI_INVOKE__('tauri', {
    __tauriModule: 'Shell',
    message: {
      cmd: 'open',
      path: loginUrl.value,
    },
  })
}

const clipboardWrite = async (a) => {
  navigator.clipboard.writeText(a)
}
</script>

<template>
  <div class="login-card">
    <img
      src="https://launcher-files.modrinth.com/assets/default_profile.png"
      class="logo"
      alt="Minecraft art"
    />
    <Card class="logging-in">
      <h2>Sign into Minecraft</h2>
      <p>
        Sign in with your Microsoft account to launch Minecraft with your mods and modpacks. If you
        don't have a Minecraft account, you can purchase the game on the
        <a
          href="https://www.minecraft.net/en-us/store/minecraft-java-bedrock-edition-pc"
          class="link"
        >
          Minecraft website
        </a>
      </p>
      <div class="action-row">
        <Button class="transparent" large @click="prevPage"> Back </Button>
        <div class="sign-in-pair">
          <Button color="primary" large @click="login">
            <LogInIcon v-if="!finalizedLogin" />
            {{ finalizedLogin ? 'Next' : 'Sign in' }}
          </Button>
        </div>
        <Button class="transparent" large @click="nextPage()"> Next </Button>
      </div>
    </Card>
  </div>
  <Modal ref="loginModal" header="Signing in" :noblur="!themeStore.advancedRendering">
    <div class="modal-body">
      <QrcodeVue :value="loginUrl" class="qr-code" margin="3" size="160" />
      <div class="modal-text">
        <div class="label">Copy this code</div>
        <div class="code-text">
          <div class="code">
            {{ loginCode }}
          </div>
          <Button
            v-tooltip="'Copy code'"
            icon-only
            large
            color="raised"
            @click="() => clipboardWrite(loginCode)"
          >
            <ClipboardCopyIcon />
          </Button>
        </div>
        <div>And enter it on Microsoft's website to sign in.</div>
        <div class="iconified-input">
          <LogInIcon />
          <input type="text" :value="loginUrl" readonly />
          <Button v-tooltip="'Open link'" icon-only color="raised" class="r-btn" @click="openUrl">
            <GlobeIcon />
          </Button>
        </div>
      </div>
    </div>
  </Modal>
</template>

<style scoped lang="scss">
.login-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  margin: auto;
  padding: var(--gap-lg);
  width: 30rem;

  img {
    width: 100%;
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }
}

.logging-in {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  vertical-align: center;
  gap: var(--gap-md);
  background-color: var(--color-raised-bg);
  width: 100%;
  border-radius: 0 0 var(--radius-lg) var(--radius-lg);

  h2,
  p {
    margin: 0;
  }

  p {
    text-align: center;
  }
}

.link {
  color: var(--color-blue);
  text-decoration: underline;
}

.button-row {
  display: flex;
  flex-direction: row;
}

.action-row {
  width: 100%;
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  gap: var(--gap-md);
  margin-top: var(--gap-md);

  .transparent {
    padding: 0 var(--gap-md);
  }
}

.qr-code {
  background-color: white !important;
  border-radius: var(--radius-md);
}

.modal-body {
  display: flex;
  flex-direction: row;
  gap: var(--gap-lg);
  align-items: center;
  padding: var(--gap-lg);

  .modal-text {
    display: flex;
    flex-direction: column;
    gap: var(--gap-sm);
    width: 100%;

    h2,
    p {
      margin: 0;
    }
  }
}

.code-text {
  display: flex;
  flex-direction: row;
  gap: var(--gap-xs);
  align-items: center;

  .code {
    background-color: var(--color-bg);
    border-radius: var(--radius-md);
    border: solid 1px var(--color-button-bg);
    font-family: var(--mono-font);
    letter-spacing: var(--gap-md);
    color: var(--color-contrast);
    font-size: 2rem;
    font-weight: bold;
    padding: var(--gap-sm) 0 var(--gap-sm) var(--gap-md);
  }

  .btn {
    width: 2.5rem;
    height: 2.5rem;
  }
}

.sticker {
  width: 100%;
  max-width: 25rem;
  height: auto;
  margin-bottom: var(--gap-lg);
}

.sign-in-pair {
  display: flex;
  flex-direction: column;
  gap: var(--gap-sm);
  align-items: center;
}
.code {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--gap-sm);

  .card {
    background: var(--color-base);
    color: var(--color-contrast);
    padding: 0.5rem 1rem;
    margin-top: 0.5rem;
  }
}
</style>
