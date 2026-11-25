🌐 [English](Settings-and-Files) · [Polski](Ustawienia-i-pliki) · [简体中文](Settings-and-Files-ZH) · **한국어**

> ⚠️ 이 페이지는 영어판을 기계 번역한 것으로, 부정확한 부분이 있을 수 있습니다. 의심스러우면 [영어판](Settings-and-Files)을 기준으로 하십시오. 수정 제안을 환영합니다.

# 설정과 파일

BaseCreator의 설정은 `app.ron` 파일 하나에 저장됩니다: 언어, 색 구성표, 그리드와 보기 옵션, 단위, 다음 페이지의 크기, 최근 파일, 측면 패널 탭의 순서와 패널이 숨겨져 있는지 여부, 창 배치, 업데이트 확인 여부. 저장하는 프로젝트에 기록되는 사람(**프로그램 옵션 → 작성자**(**Program options → Author**): 이름, 이메일, 기타)은 공유 폴더의 `author.json`에 있으며, 모든 프로그램에서 같습니다.

| 시스템 | 위치 |
|---|---|
| macOS | `~/Library/Application Support/baseCreator/apps/basecreator/app.ron` |
| Windows | `%APPDATA%\baseCreator\apps\basecreator\app.ron` |
| Linux | `$XDG_DATA_HOME/baseCreator/apps/basecreator/app.ron`, 기본값은 `~/.local/share/baseCreator/apps/basecreator/app.ron` |

* **공유 폴더.** BaseCreator, LightCreator, MillCreator, PenCreator는 데이터를 하나의 폴더 `baseCreator`에 보관합니다. 다른 폴더를 사용하려면 환경 변수 `BASECREATOR_DIR`를 설정하십시오. 그 안에는 `clipart/`(프로그램들이 공유하는 갤러리), `plugins/`, 그리고 각 프로그램의 자체 설정이 있습니다.
* **모든 것을 초기화하려면 `app.ron`을 삭제하십시오.**
* **자동 저장:** 변경 후 약 20초마다, 그리고 프로그램을 닫을 때, 파일이 있는 프로젝트는 그 파일에 저장됩니다. 아직 파일이 없는 그림은 공유 폴더의 복구 파일에 저장됩니다. 다음에 시작할 때 이런 작업을 복구하도록 제안합니다.
* **도움말 → 백업 다운로드…**(**Help → Download backup…**)는 BaseCreator의 설정과 공유 클립아트 갤러리를 zip으로 저장합니다. **도움말 → 백업에서 복원…**(**Help → Restore from backup…**)은 확인을 받은 뒤 그런 파일을 다시 읽습니다: 설정은 교체되고 그림은 갤러리에 **추가**됩니다. 다른 프로그램에서 만든 백업은 거부됩니다.
* 프로젝트(`.cre`)와 내보낸 파일은 선택한 곳에 저장됩니다.
