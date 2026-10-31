-- Usage: osascript packaging/configure-dmg.applescript <mounted-volume-path> <layout-json>
on layoutValue(layoutPath, keyName)
    return (do shell script "/usr/bin/plutil -extract " & keyName & " raw -o - " & quoted form of layoutPath) as integer
end layoutValue
on run arguments
    set volumePath to item 1 of arguments
    set layoutPath to item 2 of arguments
    set canvasWidth to layoutValue(layoutPath, "width")
    set canvasHeight to layoutValue(layoutPath, "height")
    set iconSize to layoutValue(layoutPath, "iconSize")
    set appX to layoutValue(layoutPath, "appX")
    set appY to layoutValue(layoutPath, "appY")
    set applicationsX to layoutValue(layoutPath, "applicationsX")
    set applicationsY to layoutValue(layoutPath, "applicationsY")
    set volumeFolder to POSIX file volumePath as alias
    tell application "Finder"
        set installerFolder to item volumeFolder
        open installerFolder
        delay 1
        tell container window of installerFolder
            set current view to icon view
            set toolbar visible to false
            set statusbar visible to false
            set bounds to {100, 100, 100 + canvasWidth, 128 + canvasHeight}
            set viewOptions to icon view options of container window of installerFolder
            set arrangement of viewOptions to not arranged
            set icon size of viewOptions to iconSize
            set text size of viewOptions to 16
            set background picture of viewOptions to file ".background:background.tiff" of installerFolder
            set position of item "Gatto.app" to {appX, appY}
            set position of item "Applications" to {applicationsX, applicationsY}
            close
            open
            delay 2
            close
        end tell
        update installerFolder without registering applications
    end tell
end run
