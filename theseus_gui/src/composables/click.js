import { openExternal } from '@/helpers/external'
import { onMounted } from 'vue'

const disableMiddleClick = (e) => {
  // disables middle click -> new tab
  if (e.button === 1) {
    e.preventDefault()
    // instead do a left click
    const event = new MouseEvent('click', {
      view: window,
      bubbles: true,
      cancelable: true,
    })
    e.target.dispatchEvent(event)
  }
}

const disableExternalNavigation = (e, window) => {
  let target = e.target

  while (target != null) {
    if (target.matches('a')) {
      if (
        target.href &&
        ['http://', 'https://', 'mailto:', 'tel:'].some((v) => target.href.startsWith(v)) &&
        !target.classList.contains('router-link-active') &&
        !target.href.startsWith('http://localhost') &&
        !target.href.startsWith('https://tauri.localhost')
      ) {
        openExternal(window, target.href)
      }
      e.preventDefault()
      break
    }
    target = target.parentElement
  }
}

export const useDisableClicks = (document, window) => {
  onMounted(() => {
    document
      .querySelector('body')
      .addEventListener('click', (e) => disableExternalNavigation(e, window))

    document.querySelector('body').addEventListener('auxclick', disableMiddleClick)
  })
}
