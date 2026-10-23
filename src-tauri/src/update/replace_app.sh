#!/bin/sh
# Replaces Project Hub's app bundle with the one in a downloaded disk image. Run by the update
# helper after the app has quit (see install.rs); the helper opens the app again afterwards.
# Arguments: the disk image, the bundle to replace.
# Exits non-zero when the bundle cannot be replaced (no write access, for example); the old bundle
# then stays in place. Output goes to $TMPDIR/project-hub-update.log.

dmg="$1"; app="$2"
exec >>"${TMPDIR:-/tmp}/project-hub-update.log" 2>&1
echo "$(date) updating $app from $dmg"

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
  status=0
else
  echo "could not replace the app"
  rm -rf "$staged"
  status=1
fi
hdiutil detach -quiet "$mnt" 2>/dev/null
rmdir "$mnt" 2>/dev/null
exit $status
