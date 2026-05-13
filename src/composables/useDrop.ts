import { getCurrentWebview } from '@tauri-apps/api/webview'
import { onMounted, onBeforeUnmount } from 'vue'

export interface DropPayload {
  paths: string[]
}

/**
 * Subscribe to Tauri's drag-drop events on the current webview.
 * Handler receives the absolute paths of dropped files/folders.
 */
export function useDrop(handler: (payload: DropPayload) => void) {
  let unlisten: (() => void) | null = null

  onMounted(async () => {
    const webview = getCurrentWebview()
    unlisten = await webview.onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        const paths = (event.payload.paths || []).map((p) => p.toString())
        if (paths.length) handler({ paths })
      }
    })
  })

  onBeforeUnmount(() => {
    if (unlisten) unlisten()
  })
}
