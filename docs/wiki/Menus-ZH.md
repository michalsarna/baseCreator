🌐 [English](Menus) · [Polski](Menu) · **简体中文** · [한국어](Menus-KO)

# 菜单

菜单栏有七个菜单：**文件 · 编辑 · 插入 · 排列 · 视图 · 设置 · 帮助**（**File · Edit · Insert · Arrange · View · Settings · Help**）。布局与其他程序相同，只是没有它们的 **机器**（**Machine**）菜单和加工工艺菜单。

* 在 **Windows 和 Linux** 上，菜单栏位于窗口中。它显示在编辑器中，不显示在启动界面上。
* 在 **macOS** 上，菜单位于系统菜单栏中，还有应用菜单 **BaseCreator**（**关于 BaseCreator**（**About BaseCreator**）、**退出**（**Quit**））和 **窗口**（**Window**）。
* 当前无法使用的条目显示为灰色。开关处于打开状态时带有勾号。

## 文件

| 条目 | 快捷键 | 作用 |
|---|---|---|
| **新建**（**New**） | Ctrl+N | 在独立的标签页中新建一幅空白图纸。 |
| **打开…**（**Open…**） | Ctrl+O | 在新标签页中打开项目、SVG、PDF 或 Illustrator 文件；DXF、LightBurn 或 UGS 文件则会被导入。 |
| **最近打开**（**Open recent**） | | 最近的 10 个文件；**清除列表**（**Clear list**）会清空列表。 |
| **保存**（**Save**） | Ctrl+S | 把项目写成 `.cre`。 |
| **另存为…**（**Save as…**） | Ctrl+Shift+S | 以新名称写出。 |
| **项目信息…**（**Project information…**） | | 文件、文件大小和时间、创建者、贡献者以及编辑记录。参见[项目文件](Project-File-ZH)。 |
| **页面设置…**（**Page setup…**） | | 页面大小。参见[页面与图层](Page-and-Layers-ZH)。 |
| **导入 → …**（**Import → …**） | SVG 为 Ctrl+I | 参见[导入与导出](Import-and-Export-ZH)。**生成形状…**（**Generate shape…**）运行插件的生成器。 |
| **导出 → …**（**Export → …**） | | 参见[导入与导出](Import-and-Export-ZH)。 |
| **退出**（**Quit**） | Ctrl+Q | 关闭程序；每幅有未保存内容的图纸都会先询问。 |

**打印…**（**Print…**，Ctrl+P）在纸质打印机上打印设计，或将其保存为 PDF。

## 编辑

**撤销**（**Undo**，Ctrl+Z）、**重做**（**Redo**，Ctrl+Y）、**复制**（**Copy**，Ctrl+C）、**粘贴**（**Paste**，Ctrl+V）、**再制**（**Duplicate**，Ctrl+D）、**删除**（**Delete**，Del）、**全选**（**Select all**，Ctrl+A）。

## 插入

把新对象放到页面上的工具，然后是 **生成形状…** 和 **在线剪贴画…**（**Online clipart…**）。新形状会放到当前使用的图层上。

## 排列

同样的命令也在形状的右键菜单中；它们在[设计工具](Design-Tools-ZH)中有说明。

* **左对齐**（**Align left**）、**水平居中对齐**（**Align centre (H)**）、**右对齐**（**Align right**）、**顶端对齐**（**Align top**）、**垂直居中对齐**（**Align centre (V)**）、**底端对齐**（**Align bottom**）、**在工作台上居中**（**Centre on bed**，即页面中心）、**相互居中**（**Centre on each other**）
* **水平翻转**（**Flip horizontal**）、**垂直翻转**（**Flip vertical**）、**顺时针旋转 90°**（**Rotate 90° CW**）、**逆时针旋转 90°**（**Rotate 90° CCW**）
* **编组**（**Group**，Ctrl+G）、**取消编组**（**Ungroup**，Ctrl+Shift+G）、**锁定**（**Lock**，Ctrl+L）、**解锁**（**Unlock**，Ctrl+Shift+L）、**置于顶层**（**Bring to front**）、**置于底层**（**Send to back**）
* **编辑节点**（**Edit nodes**，N）、**转换为路径**（**Convert to path**）
* **并集**（**Union**）、**交集**（**Intersection**）、**减去**（**Subtract**）、**异或**（**Exclusive or**）、**偏移形状…**（**Offset shape…**）、**圆角…**（**Round corners…**）、**转换为曲线**（**Convert to curves**）
* **自动连接路径…**（**Auto-join paths…**）、**按容差闭合路径…**（**Close path with tolerance…**）、**狗骨 / T 骨转角…**（**Dogbone / T-bone corners…**）
* **网格阵列…**（**Grid array…**）、**环形阵列…**（**Circular array…**）、**沿路径复制…**（**Copy along path…**）、**路径文字…**（**Text on path…**）
* **按形状修剪…**（**Trim by shape…**）、**删除重复项**（**Remove duplicates**）、**打散**（**Break apart**）、**简化…**（**Simplify…**）
* **序列文字…**（**Serial text…**）、**条形码 / 二维码…**（**Barcode / QR code…**）、**排样形状…**（**Nest shapes…**）、**测量…**（**Measure…**）
* **调整图像…**（**Adjust image…**）、**编辑图像…**（**Edit image…**，照片编辑器）、**描摹图像…**（**Trace image…**）
* **风格描摹…**（**Style trace…**）：以 21 种风格之一用线条重绘所选图片（[风格描摹](Style-Trace-ZH)）。
* **几何多边形…**（**Geometric polygons…**）：把所选图片变成彼此之间有间隙的几何多边形（即选中该风格的风格描摹窗口）。
* **转换为图像…**（**Convert to image…**）：把所选形状转换为一张图片。

## 视图

**网格**（**Grid**）、**吸附到网格**（**Snap to grid**）、**吸附到对象**（**Snap to objects**）、**放大**（**Zoom in**，Ctrl++）、**缩小**（**Zoom out**，Ctrl+-）、**工作台适合窗口**（**Fit bed to window**，Ctrl+Shift+0，显示整个页面）、**显示全部对象**（**Fit all objects**，Ctrl+Shift+9）、**工作区自动适合窗口**（**Auto-fit work area to window**）、**分屏视图：左右并排**（**Split view: side by side**）、**分屏视图：上下排列**（**Split view: one above the other**）、**单一视图**（**Single view**）、**背景图片…**（**Background picture…**）。

## 设置

| 条目 | 作用 |
|---|---|
| **视图选项…**（**View options…**） | 网格、工作区的颜色、线条粗细、**显示图层名称**（**Show layer names**）。 |
| **程序选项…**（**Program options…**） | **程序启动时检查新版本**（**Look for a newer version when the program starts**，默认开启），以及 **Web 服务器和 API**（**web server and API**）：一个显示项目的页面（一个随图纸更新的 SVG）和一个位于 `http://localhost:8768` 的 API（默认关闭；可设置端口、是否允许网络中的其他计算机访问，以及访问口令，初始为 `BaseCreator`，浏览器会要求输入它，用户名任意）。其他程序也有同样的服务器，端口分别为 8765（LightCreator）、8766（MillCreator）和 8767（PenCreator）。 |
| **插件…**（**Plugins…**） | 参见[插件](Plugins-ZH)。 |
| **语言**（**Language**） | 38 种语言。 |
| **配色方案**（**Colour scheme**） | 18 种配色方案，在圣诞节、新年、万圣节、复活节和春节前后另有两种节日配色方案。 |

## 帮助

**快速指南**（**Quick guide**，F1），即程序内置的指南。**GitHub 上的项目页面**（**Project page on GitHub**）、**下载备份…**（**Download backup…**）和 **从备份恢复…**（**Restore from backup…**，参见[设置与文件](Settings-and-Files-ZH)）、**检查更新…**（**Check for updates…**）、**关于**（**About**）。
