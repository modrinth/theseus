/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank Profile object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/tauri'

/// Created shared modpack from profile
export async function share_create(path) {
  return await invoke('plugin:profile_share|profile_share_create', { path })
}

/// Generates a shared profile link
export async function share_generate(path) {
  return await invoke('plugin:profile_share|profile_share_generate_share_link', { path })
}

/// Gets the shared profile from the link id
// This is done without accepting it- so would not include any link information, and is only usable for basic info
export async function share_get_link_id(link) {
  return await invoke('plugin:profile_share|profile_share_get_link_id', { link })
}

/// Accepts a shared profile link
export async function share_accept(link) {
  return await invoke('plugin:profile_share|profile_share_accept_share_link', { link })
}

/// Removes users from a shared profile
export async function remove_users(path, users) {
  return await invoke('plugin:profile_share|profile_share_remove_users', { path, users })
}

/// Removes links from a shared profile
export async function remove_links(path, links) {
  return await invoke('plugin:profile_share|profile_share_remove_links', { path, links })
}

/// Install a pack from a shared profile id
export async function share_install(sharedProfileId) {
  return await invoke('plugin:profile_share|profile_share_install', { sharedProfileId })
}

// get all user profiles that are available to the currentt user
export async function get_all(path) {
  return await invoke('plugin:profile_share|profile_share_get_all', { path })
}

// syncs profile to match that on server
export async function inbound_sync(path) {
  return await invoke('plugin:profile_share|profile_share_inbound_sync', { path })
}

// syncs profile to update server
// only allowed if profile is owned by user
export async function outbound_sync(path) {
  return await invoke('plugin:profile_share|profile_share_outbound_sync', { path })
}
