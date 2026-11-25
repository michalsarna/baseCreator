🌐 [English](Import-and-Export) · [Polski](Import-i-eksport) · **简体中文** · [한국어](Import-and-Export-KO)

> ⚠️ 本页由英文版机器翻译而成，可能有不准确之处。如有疑问，请以[英文版](Import-and-Export)为准。欢迎提出修改建议。

# 导入与导出

这些列表在 LightCreator、MillCreator、PenCreator 和 BaseCreator 中都相同。需要机器的内容不在这里：没有 G-code，3D 模型和浮雕 STL 留在 MillCreator 中。

## 导入

**文件 → 导入**（**File → Import**）中的所有内容都会添加到打开的图纸中，处于选中状态，并可通过一次撤销去除。在启动界面上则会为它新建一幅图纸。

| 条目 | 文件 | 导入的内容 |
|---|---|---|
| **导入图像…**（**Import image…**） | PNG、JPG、BMP、GIF、WebP | 一张图片，通过导入对话框导入。 |
| **导入 SVG…**（**Import SVG…**，Ctrl+I） | `.svg` | 各条路径，每条放到与其颜色最接近的图层上。 |
| **导入 Adobe Illustrator / PDF / EPS / VectorCraft…**（**Import Adobe Illustrator / PDF / EPS / VectorCraft…**） | `.pdf`、兼容 PDF 的 `.ai`、`.eps`、`.vectorcraft` | 同上。带有 Illustrator（或 VectorCraft）图层的文件会询问是按颜色还是按这些图层导入。EPS 按 PostScript 读取（读取其路径；不读取文字和图片）；由 VectorCraft 0.8 或更高版本写出的 EPS 或 `.vectorcraft` 文件会连同其中的图片一起读取。 |
| **导入 DXF…**（**Import DXF…**） | ASCII `.dxf` | 直线、圆弧和曲线，单位为毫米。DXF 的图层会放到还没有形状的图层上。 |
| **导入 LightBurn 项目…**（**Import LightBurn project…**） | `.lbrn2`、`.lbrn` | 形状。 |
| **导入 UGS 设计…**（**Import UGS design…**） | `.ugsd` | 位于各自图层上的形状。进给速度和深度会被忽略。 |
| **使用插件导入…**（**Import with a plugin…**） | 已安装插件能读取的格式 | 参见[插件](Plugins-ZH)。 |

**文件 → 打开…**（**File → Open…**）以及把文件拖放到窗口上，会为 `.cre`、SVG、PDF 或 Illustrator 文件新建一个**标签页**。DXF、LightBurn 或 UGS 文件则会导入到打开的图纸中。

## 导出

| 条目 | 写出的内容 |
|---|---|
| **导出 SVG…**（**Export SVG…**） | 所有形状，以 SVG 格式、页面大小写出。曲线会保留。 |
| **导出 PLT / DXF…**（**Export PLT / DXF…**） | 所有形状的轮廓，根据你选择的扩展名写成 HPGL（`.plt`，每个图层一支笔）或 DXF R12。 |
| **将所选内容导出为 SVG…**（**Export selection as SVG…**）/ **将所选内容导出为 DXF…**（**Export selection as DXF…**） | 所选的形状（未选中任何内容时为全部形状）。 |

导出整幅图纸时，隐藏图层上的形状也会写出。**图片没有轮廓，会被略过**，程序会说明略过了多少张；请先用 **描摹图像…**（**Trace image…**）描摹它们。

PLT 文件尚未在真实的绘图仪上试用过。参见[限制](Limitations-ZH)。
