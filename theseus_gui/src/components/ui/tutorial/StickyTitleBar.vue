<script setup>
import { Button, XIcon } from 'omorphia'
import { appWindow } from '@tauri-apps/api/window'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { window } from '@tauri-apps/api'
import { MinimizeIcon, MaximizeIcon } from '@/assets/icons'
</script>

<template>
  <div data-tauri-drag-region class="fake-appbar">
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
</template>

<style scoped lang="scss">
.fake-appbar {
  position: absolute;
  width: 100vw;
  top: 0;
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  align-items: center;
  height: 2.25rem;
  background-color: var(--color-raised-bg);
  -webkit-app-region: drag;
  z-index: 10000;
}

.window-controls {
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
    height: 2.25rem;

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
</style>
