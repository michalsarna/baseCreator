🌐 [English](Import-and-Export) · [Polski](Import-i-eksport) · [简体中文](Import-and-Export-ZH) · **한국어**

> ⚠️ 이 페이지는 영어판을 기계 번역한 것으로, 부정확한 부분이 있을 수 있습니다. 의심스러우면 [영어판](Import-and-Export)을 기준으로 하십시오. 수정 제안을 환영합니다.

# 가져오기와 내보내기

목록은 LightCreator, MillCreator, PenCreator, BaseCreator에서 같습니다. 기계가 필요한 것은 여기에 없습니다: G-code가 없으며, 3D 모델과 부조 STL은 MillCreator에만 있습니다.

## 가져오기

**파일 → 가져오기**(**File → Import**)의 모든 항목은 열린 그림에 추가되고, 선택된 상태가 되며, 한 번의 실행 취소로 사라집니다. 시작 화면에서는 이를 위해 새 그림이 만들어집니다.

| 항목 | 파일 | 들어오는 것 |
|---|---|---|
| **이미지 가져오기…**(**Import image…**) | PNG, JPG, BMP, GIF, WebP | 가져오기 대화 상자를 거친 그림. |
| **SVG 가져오기…**(**Import SVG…**, Ctrl+I) | `.svg` | 경로. 각 경로는 색이 가장 가까운 레이어에 들어갑니다. |
| **Adobe Illustrator / PDF / EPS / VectorCraft 가져오기…**(**Import Adobe Illustrator / PDF / EPS / VectorCraft…**) | `.pdf`, PDF 호환 `.ai`, `.eps`, `.vectorcraft` | 위와 같습니다. Illustrator(또는 VectorCraft) 레이어가 있는 파일은 색을 기준으로 할지 그 레이어를 기준으로 할지 묻습니다. EPS는 PostScript로 읽습니다(경로만 읽으며, 텍스트와 그림은 읽지 않습니다). VectorCraft 0.8 이상에서 저장한 EPS 또는 `.vectorcraft` 파일은 그림과 함께 읽습니다. Illustrator 레이어가 하나뿐이면 묻지 않으며, 숨겨진 Illustrator 레이어에 있는 것은 빠집니다. Illustrator나 PDF 파일 속 그림(사진: JPEG, Flate, 팔레트, 회색조, RGB, CMYK, 투명도 포함)은 제자리에 이미지로 들어옵니다. |
| **DXF 가져오기…**(**Import DXF…**) | ASCII `.dxf` | 선, 호, 곡선을 밀리미터 단위로. DXF의 레이어는 아직 도형이 없는 레이어로 들어갑니다. |
| **LightBurn 프로젝트 가져오기…**(**Import LightBurn project…**) | `.lbrn2`, `.lbrn` | 도형. |
| **UGS 디자인 가져오기…**(**Import UGS design…**) | `.ugsd` | 레이어에 놓인 도형. 이송 속도와 깊이는 무시됩니다. |
| **플러그인으로 가져오기…**(**Import with a plugin…**) | 설치된 플러그인이 읽는 형식 | [플러그인](Plugins-KO)을 참고하십시오. |

**파일 → 열기…**(**File → Open…**)와 창에 파일을 끌어다 놓기는 `.cre`, SVG, PDF 또는 Illustrator 파일을 **새 탭**으로 엽니다. DXF, LightBurn 또는 UGS 파일은 열린 그림으로 가져옵니다.

## 내보내기

| 항목 | 저장하는 것 |
|---|---|
| **SVG 내보내기…**(**Export SVG…**) | 모든 도형을 페이지 크기의 SVG로. 곡선이 유지됩니다. |
| **PLT / DXF 내보내기…**(**Export PLT / DXF…**) | 모든 도형의 윤곽선을 HPGL(`.plt`, 레이어당 펜 하나) 또는 DXF R12로, 선택한 확장자에 따라 저장합니다. |
| **선택 영역을 SVG로 내보내기…**(**Export selection as SVG…**) / **선택 영역을 DXF로 내보내기…**(**Export selection as DXF…**) | 선택한 도형(아무것도 선택하지 않았으면 전부). |

그림 전체를 내보낼 때는 숨겨진 레이어의 도형도 저장합니다. **그림에는 윤곽선이 없으므로 제외되며**, 프로그램은 몇 개가 제외되었는지 알려 줍니다. 먼저 **이미지 트레이스…**(**Trace image…**)로 트레이스하십시오.

PLT 파일은 아직 실제 플로터에서 시험해 보지 않았습니다. [제한 사항](Limitations-KO)을 참고하십시오.
