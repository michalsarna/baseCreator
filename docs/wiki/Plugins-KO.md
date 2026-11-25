🌐 [English](Plugins) · [Polski](Wtyczki) · [简体中文](Plugins-ZH) · **한국어**

> ⚠️ 이 페이지는 영어판을 기계 번역한 것으로, 부정확한 부분이 있을 수 있습니다. 의심스러우면 [영어판](Plugins)을 기준으로 하십시오. 수정 제안을 환영합니다.

# 플러그인

플러그인은 프로그램을 바꾸지 않고 기능을 추가합니다. 플러그인은 BaseCreator, LightCreator, MillCreator, PenCreator가 공유합니다. 플러그인은 공유 데이터 폴더(`baseCreator`) 안의 `plugins` 폴더에 있으며, 각 프로그램은 자신에게 맞는 부분을 사용합니다.

**설정 → 플러그인…**(**Settings → Plugins…**)은 창을 엽니다: `.zip`에서 플러그인 설치, 플러그인 켜기와 끄기, 제거, 폴더 열기.

여기서 동작하는 것:

* **가져오기 도구:** **파일 → 가져오기 → 플러그인으로 가져오기…**(**File → Import → Import with a plugin…**), 끌어다 놓기, 또는 플러그인이 읽는 확장자의 파일에 대한 **파일 → 열기…**(**File → Open…**). 도형이 그림에 추가됩니다.
* **생성기:** **파일 → 가져오기 → 도형 생성…**(**File → Import → Generate shape…**, **삽입 → 도형 생성…**(**Insert → Generate shape…**)에도 있음)은 생성기의 양식을 보여 주고 생성기가 만든 도형을 추가합니다.
* **클립아트와 번역.**

**G-code 후처리기**(`gcode-tools`, `gcode-dialects`)가 있는 플러그인은 G-code가 없으므로 여기에 설치할 수 없습니다. 공식 목록에는 *이 프로그램용이 아님*(*Not for this program*)으로 표시되며, `.zip`은 거부됩니다. 다른 프로그램이 공유 폴더에 설치한 이런 플러그인은 같은 문구로 표시되고, 여기서는 아무것도 하지 않으며, 그 프로그램을 위해 남아 있습니다. 여기서 제거하면 다른 프로그램에서도 제거됩니다.

**안전.** 플러그인 스크립트(Rhai)는 파일, 네트워크, 시스템에 접근할 수 없는 샌드박스에서 시간과 메모리 제한을 두고 실행됩니다. 신뢰하는 플러그인만 설치하십시오.

## 공식 플러그인

**설정 → 플러그인… → 공식 플러그인**(**Settings → Plugins… → Official plugins**)은 [baseCreator-plugins](https://github.com/michalsarna/baseCreator-plugins)에서 관리하는 목록을 보여 줍니다. **설치**(**Install**)는 플러그인을 체크섬을 확인하며 공유 플러그인 폴더로 다운로드합니다.

직접 만들기: [작성자 안내서](https://github.com/michalsarna/baseCreator-plugins/blob/main/docs/GUIDE.md)와 [PLUGINS.md](https://github.com/michalsarna/baseCreator/blob/main/docs/PLUGINS.md).
