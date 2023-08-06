# rimmods
A CLI utility to download rimworld mods using SteamCMD

# Usage
```
rimmods -m <path/to/rimworld/mods/folder> -s <path/to/steam/download/folder>
```

# Mod File Format
rimmods will search a file named mods.txt inside RimWorld's mod directory.

The format of mods.txt is as follows
```
<steam_workshop_url> <Mod Name>
```

Example:
```
https://steamcommunity.com/sharedfiles/filedetails/?id=2009463077 Harmony
https://steamcommunity.com/workshop/filedetails/?id=818773962 HugsLib
```
