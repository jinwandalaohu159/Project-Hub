/** Where a floating panel hanging from an anchor (a dropdown's button) goes, in window pixels. */

/** Which side of the anchor the panel opens on, and how tall it may get there. */
export interface PanelSide {
  upward: boolean;
  maxHeight: number;
}

const VIEWPORT_PADDING = 8;
/** Below this much room underneath, the panel opens upward if there is more room above. */
const MIN_ROOM_BELOW = 160;
const GAP = 4;

/** Below the anchor, or above it when there is little room below and more above. */
export function choosePanelSide(anchor: DOMRect, maxHeight: number): PanelSide {
  const below = window.innerHeight - anchor.bottom - VIEWPORT_PADDING;
  const above = anchor.top - VIEWPORT_PADDING;
  const upward = below < MIN_ROOM_BELOW && above > below;
  return { upward, maxHeight: Math.min(maxHeight, upward ? above : below) - GAP };
}

/** Fixed-position style that lines the panel up with the anchor, as wide as it. */
export function panelPosition(anchor: DOMRect, side: PanelSide): Record<string, string> {
  return {
    left: `${anchor.left}px`,
    width: `${anchor.width}px`,
    maxHeight: `${side.maxHeight}px`,
    ...(side.upward
      ? { bottom: `${window.innerHeight - anchor.top + GAP}px` }
      : { top: `${anchor.bottom + GAP}px` }),
  };
}

/** True while some part of `element` is inside the window and every scrolling box around it. */
export function isScrolledIntoView(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  let top = 0;
  let left = 0;
  let bottom = window.innerHeight;
  let right = window.innerWidth;
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    const style = getComputedStyle(parent);
    if (!/(auto|scroll|hidden|clip)/.test(`${style.overflowX} ${style.overflowY}`)) continue;
    const box = parent.getBoundingClientRect();
    top = Math.max(top, box.top);
    left = Math.max(left, box.left);
    bottom = Math.min(bottom, box.bottom);
    right = Math.min(right, box.right);
  }
  return rect.bottom > top && rect.top < bottom && rect.right > left && rect.left < right;
}
