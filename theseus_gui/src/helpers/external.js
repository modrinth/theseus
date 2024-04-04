export const openExternal = (window, url) => {
  window.__TAURI_INVOKE__('tauri', {
    __tauriModule: 'Shell',
    message: {
      cmd: 'open',
      path: url,
    },
  })
}
