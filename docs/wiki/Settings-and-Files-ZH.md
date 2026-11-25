🌐 [English](Settings-and-Files) · [Polski](Ustawienia-i-pliki) · **简体中文** · [한국어](Settings-and-Files-KO)

# 设置与文件

BaseCreator 的设置保存在一个文件 `app.ron` 中：语言、配色方案、网格和视图选项、单位、下一个页面的尺寸、最近使用的文件、侧边面板中标签页的顺序以及面板是否隐藏、窗格布局，以及是否检查更新。你保存的项目中记录的人员信息（**程序选项 → 作者**（**Program options → Author**）：姓名、电子邮件、其他）保存在共享文件夹的 `author.json` 中，所有程序通用。

| 系统 | 位置 |
|---|---|
| macOS | `~/Library/Application Support/baseCreator/apps/basecreator/app.ron` |
| Windows | `%APPDATA%\baseCreator\apps\basecreator\app.ron` |
| Linux | `$XDG_DATA_HOME/baseCreator/apps/basecreator/app.ron`，默认为 `~/.local/share/baseCreator/apps/basecreator/app.ron` |

* **共享文件夹。** BaseCreator、LightCreator、MillCreator 和 PenCreator 把数据保存在同一个文件夹 `baseCreator` 中。设置环境变量 `BASECREATOR_DIR` 可以使用其他文件夹。其中有 `clipart/`（各程序共享的图库）、`plugins/` 以及每个程序自己的设置。
* **删除 `app.ron` 即可重置所有设置。**
* **自动保存**：每次更改后约每 20 秒以及程序关闭时，已有文件的项目会写入该文件；还没有文件的图纸会写入共享文件夹中的恢复文件。下次启动时会提供恢复这些工作。
* **帮助 → 下载备份…**（**Help → Download backup…**）把 BaseCreator 的设置和共享剪贴画库保存为一个 zip 文件。**帮助 → 从备份恢复…**（**Help → Restore from backup…**）在确认后读回这样的文件：设置会被替换，图片会被**添加**到图库中。由其他程序制作的备份会被拒绝。
* 项目（`.cre`）和导出的文件保存在你选择的任何位置。
