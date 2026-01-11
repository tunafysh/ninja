import { check } from "@tauri-apps/plugin-updater"
import { relaunch } from "@tauri-apps/plugin-process"

export async function downloadUpdate(
  onProgress: (percent: number) => void
) {
  const update = await check()
  if (!update) return

  let downloaded = 0
  let contentLength = 0

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        if(event.data.contentLength != undefined) contentLength = event.data.contentLength
        onProgress(0)
        break

      case "Progress":
        downloaded += event.data.chunkLength
        if (contentLength > 0) {
          const percent = Math.min(
            Math.round((downloaded / contentLength) * 100),
            100
          )
          onProgress(percent)
        }
        break

      case "Finished":
        onProgress(100)
        break
    }
  })

  await relaunch()
}
