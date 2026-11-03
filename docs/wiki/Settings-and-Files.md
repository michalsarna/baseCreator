🌐 **English** · [Polski](Ustawienia-i-pliki)

# Settings and Files

The settings of BaseCreator are stored in one file, `app.ron`: the language, the colour scheme, the grid and the view options, the units, the size of the next page, the recent files, the order of the tabs of the side panel and whether it is hidden, the layout of the panes, and whether to look for updates.

| System | Location |
|---|---|
| macOS | `~/Library/Application Support/baseCreator/apps/basecreator/app.ron` |
| Windows | `%APPDATA%\baseCreator\apps\basecreator\app.ron` |
| Linux | `$XDG_DATA_HOME/baseCreator/apps/basecreator/app.ron`, by default `~/.local/share/baseCreator/apps/basecreator/app.ron` |

* **Shared folder.** BaseCreator, LightCreator, MillCreator and PenCreator keep their data in one folder, `baseCreator`. Set the environment variable `BASECREATOR_DIR` to use another folder. In it are `clipart/` (the gallery, shared by the programs), `plugins/` and each program's own settings.
* **Delete `app.ron` to reset everything.**
* **Automatic saving:** about every 20 seconds after a change, and when the program closes, a project that has a file is written to that file; a drawing that has no file yet is written to a recovery file in the shared folder. The next start offers such work.
* **Help → Download backup…** saves a zip with the settings of BaseCreator and the shared clipart gallery. **Help → Restore from backup…** reads such a file back after a confirmation: the settings are replaced and the pictures are **added** to the gallery. A backup made by another program is refused.
* Projects (`.cre`) and exported files are saved wherever you choose.
