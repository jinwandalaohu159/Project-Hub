#!/bin/sh
# Replaces Project Hub's app bundle with the one in a downloaded disk image, then opens it again.
# Started by the app right before it quits (see install.rs).
# Arguments: the app's process id, the disk image, the bundle to replace.
# When the bundle cannot be replaced (no write access, for example) the old one stays and the disk
# image is opened for the user instead. Output goes to $TMPDIR/project-hub-update.log.

pid="$1"; dmg="$2"; app="$3"
exec >>"${TMPDIR:-/tmp}/project-hub-update.log" 2>&1
echo "$(date) updating $app from $dmg"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done

staged="$app.update"; old="$app.old"; mnt="$(mktemp -d)"
replace() {
  hdiutil attach -nobrowse -noautoopen -readonly -mountpoint "$mnt" "$dmg" || return 1
  new="$(ls -d "$mnt"/*.app 2>/dev/null | head -n 1)"
  rm -rf "$staged" "$old"
  [ -n "$new" ] && ditto "$new" "$staged" || return 1
  mv "$app" "$old" || return 1
  if ! mv "$staged" "$app"; then mv "$old" "$app"; return 1; fi
  rm -rf "$old"
}

if replace; then
  hdiutil detach -quiet "$mnt"
  open "$app"
else
  echo "could not replace the app; opening the disk image"
  rm -rf "$staged"
  hdiutil detach -quiet "$mnt" 2>/dev/null
  open "$dmg"
fi
rmdir "$mnt" 2>/dev/null
