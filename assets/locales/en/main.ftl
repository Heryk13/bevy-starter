# English, the fallback: a key missing from another language is read from here.
# Every key here must also be in the other files; a test in src/i18n.rs checks it.

## main menu

menu-new-game = New Game
menu-continue = Continue
menu-load-game = Load Game
menu-settings = Settings
menu-credits = Credits
menu-quit = Quit

## pause menu

pause-title = Paused
pause-resume = Resume
pause-save = Save
pause-load-game = Load Game
pause-settings = Settings
pause-quit-to-menu = Quit to Menu
pause-quit-to-desktop = Quit to Desktop

## dialogs

dialog-yes = Yes
dialog-no = No
dialog-quit-without-saving = Quit without saving?
dialog-overwrite-save = Overwrite this save?
dialog-delete-save = Delete this save?

## overlays

overlay-close = Close
credits-made-with = Made with Bevy
settings-title = Settings
settings-fullscreen = Fullscreen
settings-volume = Volume
settings-language = Language

## save and load

save-game-title = Save Game
load-game-title = Load Game
save-new = New Save
save-delete = Delete
saves-none = No saves yet
save-incompatible = Incompatible Save
save-unreadable = Unreadable Save
save-unnamed = Unnamed
save-autosave = Autosave
# a row in the save lists: "Castle · 1h 01m played · 3d ago"
save-label = { $name } · { $played } · { $age }
# $minutes arrives already padded to two digits
save-played = { $hours }h { $minutes }m played
age-seconds = { $count }s ago
age-minutes = { $count }m ago
age-hours = { $count }h ago
age-days = { $count }d ago
status-saved = Saved successfully.
status-save-failed = Failed to save.
