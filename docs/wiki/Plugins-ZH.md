🌐 [English](Plugins) · [Polski](Wtyczki) · **简体中文** · [한국어](Plugins-KO)

# 插件

插件可以在不修改程序的情况下增加功能。它们由 BaseCreator、LightCreator、MillCreator 和 PenCreator 共享：插件位于共享数据文件夹（`baseCreator`）内的 `plugins` 文件夹中，每个程序使用适合自己的部分。

**设置 → 插件…**（**Settings → Plugins…**）会打开插件窗口：从 `.zip` 安装插件、启用和停用插件、删除插件、打开插件文件夹。

在这里可用的功能：

* **导入器**：**文件 → 导入 → 使用插件导入…**（**File → Import → Import with a plugin…**）、拖放，或对插件能读取其扩展名的文件使用 **文件 → 打开…**（**File → Open…**）。形状会添加到图纸中。
* **生成器**：**文件 → 导入 → 生成形状…**（**File → Import → Generate shape…**，也可用 **插入 → 生成形状…**（**Insert → Generate shape…**））显示生成器的表单，并添加它生成的形状。
* **剪贴画和翻译。**

带有 **G-code 后处理器**（`gcode-tools`、`gcode-dialects`）的插件不能在这里安装，因为这里没有 G-code：官方列表会将其标为 *不适用于此程序*（*Not for this program*），`.zip` 也会被拒绝。由其他程序安装到共享文件夹中的此类插件会显示同样的文字，在这里不起作用，并为那个程序保留。在这里删除它，其他程序中也会一并删除。

**安全性。** 插件脚本（Rhai）在沙箱中运行，无法访问文件、网络或系统，并有时间和内存限制。只安装你信任的插件。

## 官方插件

**设置 → 插件… → 官方插件**（**Settings → Plugins… → Official plugins**）显示 [baseCreator-plugins](https://github.com/michalsarna/baseCreator-plugins) 中维护的列表。**安装**（**Install**）会下载插件并校验其校验和，然后放入共享插件文件夹。

编写自己的插件：参见[作者指南](https://github.com/michalsarna/baseCreator-plugins/blob/main/docs/GUIDE.md)和 [PLUGINS.md](https://github.com/michalsarna/baseCreator/blob/main/docs/PLUGINS.md)。
