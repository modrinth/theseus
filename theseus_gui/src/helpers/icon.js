import { convertFileSrc } from '@tauri-apps/api/tauri'

export const iconPathAsUrl = (iconPath) => {
  if (!iconPath) {
    return ''
  }
  const startsWithHttp = iconPath.startsWith('http')
  if (startsWithHttp) {
    return iconPath
  }
  return convertFileSrc(iconPath)
}
