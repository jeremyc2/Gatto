-- Usage: osascript packaging/configure-dmg.applescript <mounted-volume-path>
-- These coordinates correspond to render-dmg-background.swift's 840 x 520 canvas.
on run arguments
    set volumePath to item 1 of arguments
    set volumeFolder to POSIX file volumePath as alias
    tell application "Finder"
        set installerFolder to item volumeFolder
        open installerFolder
        delay 1
        tell container window of installerFolder
            set current view to icon view
            set toolbar visible to false
            set statusbar visible to false
            set bounds to {100, 100, 940, 648}
            set viewOptions to icon view options of container window of installerFolder
            set arrangement of viewOptions to not arranged
            set icon size of viewOptions to 112
            set text size of viewOptions to 16
            set background picture of viewOptions to file ".background:background.tiff" of installerFolder
            set position of item "Gatto.app" to {125, 315}
            set position of item "Applications" to {715, 315}
            close
            open
            delay 2
            close
        end tell
        update installerFolder without registering applications
    end tell
end run
