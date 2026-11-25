🌐 [English](Architecture) · [Polski](Architektura) · [简体中文](Architecture-ZH) · **한국어**

> ⚠️ 이 페이지는 영어판을 기계 번역한 것으로, 부정확한 부분이 있을 수 있습니다. 의심스러우면 [영어판](Architecture)을 기준으로 하십시오. 수정 제안을 환영합니다.

# 아키텍처

BaseCreator는 라이브러리 저장소 안에 있는 하나의 바이너리 크레이트 `crates/basecreator`입니다. 라이브러리는 `base-core`(기하, 장면, SVG / PDF / DXF / LightBurn / UGS 리더, 프로젝트 컨테이너)와 `base-ui`(창, 메뉴, 편집기, 창과 탭, 클립아트, 플러그인)입니다. LightCreator, MillCreator, PenCreator도 같은 라이브러리를 git 태그로 고정하여 사용합니다.

| 파일 | 담고 있는 것 |
|---|---|
| `main.rs` | 애플리케이션: 열기, 저장, 가져오기, 시작 화면, 막대, 측면 패널, 키, 창. |
| `menu.rs` | 메뉴 항목(배치는 base-ui의 `standard_menus`). |
| `native_menu.rs` | macOS 메뉴 막대. |
| `workspaces.rs` | 탭으로 열린 그림. |
| `panes_glue.rs` | 작업 영역의 창. |
| `section.rs` | 프로젝트 파일의 `draw` 섹션: 페이지와 레이어. |
| `page_ui.rs` | **페이지 설정…**(**Page setup…**) |
| `layers_ui.rs` | 레이어 탭. |
| `plugins_glue.rs`, `autosave_glue.rs`, `backup_ui.rs`, `update_glue.rs` | 플러그인, 자동 저장, 백업, 업데이트 확인을 프로그램에 연결하는 부분. |
| `texts.rs` | BaseCreator 자체의 번역. |
| `screenshots.rs` | 그림을 렌더링하는 부분. |
| `tests.rs` | 프로그램의 테스트. |
