import { computed, ref, watch, type CSSProperties } from 'vue'
import { usePersistentRef, clamp } from './usePersistentRef'
import { useProjectEvents } from './useProjectEvents'
import { morphLayout } from '../utils/viewTransition'

const DEFAULT_LEFT_WIDTH = 300
const DEFAULT_MIDDLE_WIDTH = 280
const DEFAULT_VERTICAL_SPLIT = 66.3
const MIN_PANEL_WIDTH = 160
const MAX_PANEL_WIDTH = 900
/** Stacked, the project panel gives up this much of its share so the file panel's title fits. */
const STACKED_FILE_OFFSET_PX = 10
/** Width (side by side) or height (stacked) of a collapsed panel. */
const COLLAPSED_PX = 40

export type LeftLayout = 'horizontal' | 'vertical'

/**
 * Sizes and arrangement of the main window's panels: the project and file panels side by side
 * or stacked, their widths, the stacked split and which panels are collapsed. Sizes persist.
 */
export function usePanelLayout() {
  const { workingTreeChangeCount } = useProjectEvents()

  const leftWidth = usePersistentRef('left-width', DEFAULT_LEFT_WIDTH)
  const middleWidth = usePersistentRef('middle-width', DEFAULT_MIDDLE_WIDTH)

  const leftCollapsed = usePersistentRef('left-collapsed', false)
  const middleCollapsed = usePersistentRef('middle-collapsed', false)
  const rightCollapsed = usePersistentRef('right-collapsed', false)

  const leftLayout = usePersistentRef<LeftLayout>('left-layout', 'horizontal')

  const leftCollapsedVertical = ref(false)
  const middleCollapsedVertical = ref(false)
  const autoCollapsedFilesForChanges = ref(false)
  const filePanelWasCollapsedBeforeChanges = ref(false)

  const verticalSplit = usePersistentRef('vertical-split', DEFAULT_VERTICAL_SPLIT) // percentage, project panel gets more space
  const verticalContainerRef = ref<HTMLElement | null>(null)

  /** Panels may grow until the code view would drop below a usable width. */
  function maxPanelWidth(other: number) {
    const available = typeof window === 'undefined' ? MAX_PANEL_WIDTH : window.innerWidth - other - 240
    return Math.max(MIN_PANEL_WIDTH, Math.min(MAX_PANEL_WIDTH, available))
  }

  function onLeftResize(dx: number) {
    const other = leftLayout.value === 'horizontal' && !middleCollapsed.value ? middleWidth.value : 0
    leftWidth.value = clamp(leftWidth.value + dx, MIN_PANEL_WIDTH, maxPanelWidth(other))
  }

  function onMiddleResize(dx: number) {
    const other = leftCollapsed.value ? 40 : leftWidth.value
    middleWidth.value = clamp(middleWidth.value + dx, MIN_PANEL_WIDTH, maxPanelWidth(other))
  }

  function onVerticalResize(dy: number) {
    const containerHeight = verticalContainerRef.value?.clientHeight ?? 600
    const deltaPercent = (dy / containerHeight) * 100
    verticalSplit.value = clamp(verticalSplit.value + deltaPercent, 15, 85)
  }

  function resetLeftWidth() {
    leftWidth.value = DEFAULT_LEFT_WIDTH
  }

  function resetMiddleWidth() {
    middleWidth.value = DEFAULT_MIDDLE_WIDTH
  }

  function resetVerticalSplit() {
    verticalSplit.value = DEFAULT_VERTICAL_SPLIT
  }

  // Both arrangements render the same panels; these pick each panel's collapse state and size for
  // the current one, so switching moves the panels instead of rebuilding them.
  const stacked = computed(() => leftLayout.value === 'vertical')

  const projectCollapsed = computed({
    get: () => (stacked.value ? leftCollapsedVertical.value : leftCollapsed.value),
    set: (value: boolean) => {
      if (stacked.value) leftCollapsedVertical.value = value
      else leftCollapsed.value = value
    },
  })

  const filesCollapsed = computed({
    get: () => (stacked.value ? middleCollapsedVertical.value : middleCollapsed.value),
    set: (value: boolean) => {
      if (stacked.value) middleCollapsedVertical.value = value
      else middleCollapsed.value = value
    },
  })

  const projectPanelStyle = computed<CSSProperties>(() => {
    if (!stacked.value) return { width: `${leftCollapsed.value ? COLLAPSED_PX : leftWidth.value}px` }
    if (leftCollapsedVertical.value) return { height: `${COLLAPSED_PX}px` }
    if (middleCollapsedVertical.value) return { flex: 1 }
    return { height: `calc(${verticalSplit.value}% - ${STACKED_FILE_OFFSET_PX}px)` }
  })

  const filesPanelStyle = computed<CSSProperties>(() => {
    if (!stacked.value) return { width: `${middleCollapsed.value ? COLLAPSED_PX : middleWidth.value}px` }
    return middleCollapsedVertical.value ? { height: `${COLLAPSED_PX}px` } : { flex: 1 }
  })

  /** The panels morph to the other arrangement rather than jumping. */
  function toggleLeftLayout() {
    morphLayout(() => {
      leftLayout.value = leftLayout.value === 'horizontal' ? 'vertical' : 'horizontal'
    })
  }

  // Stacked, the file panel folds away while there are unsaved changes so the change list has room,
  // and comes back as it was once they are saved.
  watch([leftLayout, workingTreeChangeCount], ([layout, changeCount]) => {
    const shouldCollapseFiles = layout === 'vertical' && changeCount > 0

    if (shouldCollapseFiles && !autoCollapsedFilesForChanges.value) {
      filePanelWasCollapsedBeforeChanges.value = middleCollapsedVertical.value
      middleCollapsedVertical.value = true
      autoCollapsedFilesForChanges.value = true
      return
    }

    if (!shouldCollapseFiles && autoCollapsedFilesForChanges.value) {
      middleCollapsedVertical.value = filePanelWasCollapsedBeforeChanges.value
      autoCollapsedFilesForChanges.value = false
    }
  })

  return {
    leftWidth,
    middleWidth,
    leftCollapsed,
    middleCollapsed,
    rightCollapsed,
    leftLayout,
    stacked,
    projectCollapsed,
    filesCollapsed,
    projectPanelStyle,
    filesPanelStyle,
    leftCollapsedVertical,
    middleCollapsedVertical,
    verticalSplit,
    verticalContainerRef,
    onLeftResize,
    onMiddleResize,
    onVerticalResize,
    resetLeftWidth,
    resetMiddleWidth,
    resetVerticalSplit,
    toggleLeftLayout,
  }
}
