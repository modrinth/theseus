import {
  add_project_from_version as installMod,
  check_installed,
  get_full_path,
  get_mod_full_path,
} from '@/helpers/profile'
import { useFetch } from '@/helpers/fetch.js'
import { handleError } from '@/store/notifications.js'
import { invoke } from '@tauri-apps/api/tauri'

export async function isDev() {
  return await invoke('is_dev')
}

/// Show Main Window - called once after first frontend rendered
export async function showMainWindow() {
  return await invoke('show_window')
}

// One of 'Windows', 'Linux', 'MacOS'
export async function getOS() {
  return await invoke('plugin:utils|get_os')
}

export async function showInFolder(path) {
  return await invoke('plugin:utils|show_in_folder', { path })
}

export async function showLauncherLogsFolder() {
  return await invoke('plugin:utils|show_launcher_logs_folder', {})
}

// Opens a profile's folder in the OS file explorer
export async function showProfileInFolder(path) {
  const fullPath = await get_full_path(path)
  return await showInFolder(fullPath)
}

export async function highlightModInProfile(profilePath, projectPath) {
  const fullPath = await get_mod_full_path(profilePath, projectPath)
  return await showInFolder(fullPath)
}

export const releaseColor = (releaseType) => {
  switch (releaseType) {
    case 'release':
      return 'green'
    case 'beta':
      return 'orange'
    case 'alpha':
      return 'red'
    default:
      return ''
  }
}

export const installVersionDependencies = async (profile, version) => {
  for (const dep of version.dependencies) {
    if (dep.dependency_type !== 'required') continue
    // disallow fabric api install on quilt
    if (dep.project_id === 'P7dR8mSH' && profile.metadata.loader === 'quilt') continue
    if (dep.version_id) {
      if (
        dep.project_id &&
        (await check_installed(profile.path, dep.project_id).catch(handleError))
      )
        continue
      await installMod(profile.path, dep.version_id)
    } else {
      if (
        dep.project_id &&
        (await check_installed(profile.path, dep.project_id).catch(handleError))
      )
        continue
      const depVersions = await useFetch(
        `https://api.modrinth.com/v2/project/${dep.project_id}/version`,
        'dependency versions',
      )
      const latest = depVersions.find(
        (v) =>
          v.game_versions.includes(profile.metadata.game_version) &&
          v.loaders.includes(profile.metadata.loader),
      )
      if (latest) {
        await installMod(profile.path, latest.id).catch(handleError)
      }
    }
  }
}

export const openLink = (url) => {
  window.__TAURI_INVOKE__('tauri', {
    __tauriModule: 'Shell',
    message: {
      cmd: 'open',
      path: url,
    },
  })
}

export const refreshOffline = async () => {
  return await invoke('plugin:utils|refresh_offline', {})
}

// returns true/false
export const isOffline = async () => {
  return await invoke('plugin:utils|is_offline', {})
}
