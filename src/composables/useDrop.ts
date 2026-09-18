import { onMounted, onBeforeUnmount } from 'vue'

export interface DropPayload {
  paths: string[]
}

function hasTauriInternals() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/**
 * Subscribe to Tauri's drag-drop events on the current webview.
 * Handler receives the absolute paths of dropped files/folders.
 */
export function useDrop(handler: (payload: DropPayload) => void) {
  let unlisten: (() => void) | null = null

  onMounted(async () => {
    if (!hasTauriInternals()) {
      console.warn('[useDrop] skipped: no Tauri internals (browser or IPC missing)')
      return
    }
    try {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview')
      const webview = getCurrentWebview()
      unlisten = await webview.onDragDropEvent((event) => {
        if (event.payload.type === 'drop') {
          const paths = (event.payload.paths || []).map((p) => p.toString())
          if (paths.length) handler({ paths })
        }
      })
    } catch (e) {
      console.warn('[useDrop] subscribe failed:', e)
    }
  })

  onBeforeUnmount(() => {
    if (unlisten) unlisten()
  })
}
