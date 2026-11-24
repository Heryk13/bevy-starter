# English, the fallback: a key missing from another language is read from here.
# Every key here must also be in the other files; a test in src/i18n.rs checks it.

## main menu

menu-new-game = New Game
menu-continue = Continue
menu-load-game = Load Game
menu-find-server = Find Server
menu-settings = Settings
menu-credits = Credits
menu-quit = Quit

## pause menu

pause-title = Paused
pause-resume = Resume
pause-save = Save
pause-load-game = Load Game
pause-settings = Settings
pause-open-to-lan = Open to LAN
pause-close-lan = Close LAN
pause-quit-to-menu = Quit to Menu
pause-quit-to-desktop = Quit to Desktop

## dialogs

dialog-yes = Yes
dialog-no = No
dialog-ok = OK
dialog-quit-without-saving = Quit without saving?
dialog-overwrite-save = Overwrite this save?
dialog-delete-save = Delete this save?
dialog-reset-controls = Reset all controls to their defaults?
notice-could-not-join = Could not join that game.
notice-host-ended = The host ended the game.
notice-open-to-lan = Your game is open to LAN. Friends on your network can join it from Find Server.

## overlays

overlay-close = Close

## find server

find-server-title = Find Server
find-server-refresh = Refresh
find-server-empty = No games found on the network.
find-server-by-ip = Join by IP
find-server-join = Join
credits-title = Credits
credits-made-with = Made with Bevy
settings-title = Settings
settings-fullscreen = Fullscreen
settings-reduce-transparency = Reduce Transparency
settings-volume = Volume
settings-language = Language
settings-page-general = General
settings-page-controls = Controls
controls-listening = Press a key…
controls-cancel-hint = Esc cancels
controls-reset = Reset to Defaults
action-move-up = Move Up
action-move-down = Move Down
action-move-left = Move Left
action-move-right = Move Right

## save and load

save-game-title = Save Game
load-game-title = Load Game
save-new = New Save
saves-none = No saves yet
save-incompatible = Incompatible Save
save-unreadable = Unreadable Save
save-unnamed = Unnamed
save-autosave = Autosave
# under a save's name in the lists: "1h 01m played · 3d ago"
save-details = { $played } · { $age }
# $minutes arrives already padded to two digits
save-played = { $hours }h { $minutes }m played
age-seconds = { $count }s ago
age-minutes = { $count }m ago
age-hours = { $count }h ago
age-days = { $count }d ago
status-saved = Saved successfully.
status-save-failed = Failed to save.
