# RGB Switch

이 PC의 RGB의 ON/OFF·색상·애니메이션을 제어하는 Windows Tauri 2 + React/TypeScript 앱입니다. OpenRGB 실행 파일이나 서버 없이 Rust 장치 모듈이 직접 통신합니다. 검정·진회색 화면, 분홍/다홍/주황/노랑 그라데이션과 커스텀 제목 표시줄을 제공합니다. 전체 폰트는 앱에 포함된 Pretendard Variable이며 오프라인에서도 사용됩니다. 글꼴 라이선스는 `public/licenses/Pretendard-OFL.txt`에 포함했습니다.

## 지원 범위

| 장치 | 조명 설정 | 검증 |
| --- | --- | --- |
| HARPOON RGB WIRELESS (USB 유선) | 로고 단색·ON/OFF, 5단계 DPI 수치·표시등 색상 | 단색 유지와 DPI 버튼·표시등·감도 변화를 사용자 확인. 사용자 지정 감도·단계 재조회 및 종료 복원 |
| Palit RTX 3070 | 단색·숨쉬기·색상 순환, 밝기·속도 | 세 효과의 조명 레지스터 재조회 및 복원 |
| MSI B860M MORTAR WIFI | 단색·숨쉬기·색상 순환, 밝기·속도 | Gen1 포트 초기화 후 실제 본체 팬 소등을 사용자가 확인 |
| GANSS GS3087T (USB 유선) | 단색·숨쉬기·색상 순환, 밝기·속도 | 명령 ACK, 세 효과의 렌더링 색상 조회 |
| CORSAIR VENGEANCE RGB DDR5 128 GB (2개 묶음) | 단색·숨쉬기·색상 순환, 밝기·속도 | 두 모듈의 세 효과 CRC·재조회 및 복원 |

장치 이름이나 펼침 화살표를 누르면 그 장치의 색상·밝기·효과 설정이 바로 아래에 나타납니다. 편집 중인 값은 장치를 접거나 다른 장치를 펼쳐도 유지되며 **조명 적용**은 해당 기기만 변경합니다. 마우스는 단색만 지원합니다. 상단 전체 ON/OFF는 제어 가능한 모든 장치에 적용합니다. 이번 실행에서 적용한 조명은 OFF 후 ON으로 다시 켤 수 있습니다. 밝기 0% 설정에서 ON을 누르면 100%로 켭니다. 조명 설정은 재실행 시 초기화되며 자동 적용하지 않습니다.

마우스를 펼친 뒤 **DPI · 표시등**에서 5단계의 사용 여부, DPI 수치(200~10000, 200 단위), 표시등 색상과 적용 단계를 정한 뒤 **DPI 저장·적용**을 누르세요. DPI 설정은 앱 설정 폴더의 `mouse-dpi.json`에 저장합니다. 마우스의 기존 프리셋이나 버튼 배치는 덮어쓰지 않습니다. 앱 실행 중 DPI 버튼을 처리하고 해당 단계의 표시등만 갱신하므로 로고 단색이 유지됩니다. 앱을 종료하면 현재 단계에 대응하는 기존 마우스 DPI와 하드웨어 조명 모드로 돌아갑니다. 재실행 후 마우스 조명 또는 DPI를 적용하면 저장한 DPI 설정을 사용합니다. DPI를 먼저 적용하면 로고는 분홍색 70%로 시작합니다. iCUE가 실행 중이면 충돌 안내를 표시하며, 다른 앱을 강제로 종료하지 않습니다.

효과는 마우스를 제외한 각 장치의 하드웨어에서 재생합니다. 장치별 밝기·속도 단계가 달라 실제 밝기와 주기는 동일하지 않으며, 프레임 단위 동기화는 제공하지 않습니다.
메인보드에 연결된 본체 팬은 **Gen1 ARGB 포트 초기화 후 실제 소등을 확인했습니다.** 제어할 때 네 포트에서 Gen2 장치가 없는지 확인한 뒤 일반 ARGB 모드를 설정합니다. OFF는 컨트롤러의 데이터 출력을 유지한 채 각 영역에 꺼짐 효과를 보냅니다. 이 초기화를 생략한 이전 방식은 설정값만 OFF로 바뀌고 팬은 계속 켜져 있었습니다. 각 제어 때 포트 확인에 약 2~3초가 걸립니다. 팬 속도나 장치 전원은 바꾸지 않습니다.

메인보드는 OFF 전 효과를 이번 실행 동안 기억합니다. 반복 OFF로 덮어쓰지 않으며, ON 시 복원합니다. 앱을 다시 실행해 복원 기록이 없고 현재 효과가 꺼짐이면 흰색 단색으로 켭니다. Gen2 장치가 새로 감지되면 이 PC용 Gen1 전환을 중단하고 오류를 표시합니다.

최근 명령은 통신 결과이며 광학적으로 측정한 발광 상태가 아닙니다. 마우스는 종료 시 하드웨어 모드로 복원하며, 다른 장치는 종료 시 이전 효과를 자동 복구하지 않습니다. 다른 조명 프로그램이 설정을 덮어쓸 수 있습니다.

## 실행

`src-tauri/target/release/rgb-switch.exe`를 실행하고 UAC 관리자 권한 요청을 승인한 뒤 **장치 확인하기**를 누르세요. 전체 버튼은 제어 가능한 항목만 대상으로 합니다.

첫 실행 시 현재 사용자의 Windows 로그인 자동 시작을 등록합니다. 로그인 15초 뒤 `--background`로 실행되어 창을 표시하지 않고 트레이에 상주합니다. 메모리 제어에 필요한 관리자 권한을 유지하도록 Windows 작업 스케줄러의 사용자별 작업을 사용합니다. 앱 하단 **Windows 로그인 시 트레이에서 시작**을 해제하면 등록이 제거되며, 선택은 `startup.json`에 저장됩니다. EXE를 이동했다면 새 위치에서 한 번 실행해 등록 경로를 갱신하세요.

닫기 버튼과 Alt+F4는 창만 숨깁니다. 트레이 아이콘을 클릭하거나 메뉴의 **RGB Switch 열기**를 선택하면 창이 열립니다. 완전히 끝내려면 트레이 메뉴의 **종료**를 선택하세요. 이미 실행 중일 때 EXE를 다시 열면 기존 창을 표시합니다. 트레이에 있는 동안 적용한 마우스 DPI 제어도 계속됩니다.

Windows x64, WebView2 Runtime, 설치된 NVIDIA 그래픽 드라이버를 사용합니다. 메모리용 Corsair DLL·서명된 SYS·원본 EULA를 **이 PC 전용 EXE 안에 포함**했습니다. 메모리 접근 시 `C:\Program Files\RGB Switch\drivers\corsair-de834e26-020acbf0`에 추출하고 SHA-256을 검증한 뒤 전용 수동 시작 커널 드라이버 서비스를 등록·시작합니다. 기존 iCUE 서비스는 변경하지 않습니다. iCUE 설치 폴더에서 파일을 읽거나 OpenRGB 서버를 실행할 필요가 없습니다.

이 빌드의 Corsair 파일은 사용자의 기존 설치본에서 변경 없이 가져왔습니다. 개인 사용 범위의 빌드이며 타인에게 재배포할 권리를 확인한 패키지는 아닙니다. 원본 라이선스와 바이너리는 `src-tauri/private-driver/`에 있으며 버전 관리에서 제외했습니다. Windows가 드라이버를 차단하면 오류를 표시하며 보안 설정을 변경하지 않습니다.

## 개발

Node.js, Rust MSVC, Visual Studio C++ Build Tools가 필요합니다.

```powershell
npm ci
pwsh -NoProfile -File tools/prepare-private-driver.ps1 # 새 체크아웃의 개인 빌드 입력 준비
npm run dev        # 브라우저 UI / 데모, 실제 RGB 접근 없음
npm run desktop    # 실제 Tauri 개발 앱
npm run build      # TypeScript 검사 + 프런트엔드 빌드
npm run exe        # EXE 빌드
npm run installer  # NSIS 설치 패키지 (이번 검증 범위 밖)
npm run test:rust
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

- `src/`: 한국어 UI, 전체/개별 명령, 상세 오류, 데모
- `src-tauri/src/devices/`: 모델별 HID/NVAPI/SMBus 모듈
- `src-tauri/src/lib.rs`: 직렬화된 Tauri 명령과 최근 요청 상태
- `src-tauri/examples/direct_probe.rs`: 읽기 진단. `--test-mouse`, `--test-gpu`는 실제 조명 변경
- `src-tauri/examples/*_switch_test.rs`: 실제 OFF/ON 테스트. 메인보드와 RAM은 테스트 전 설정 복원, 키보드는 흰색으로 종료
- [HARDWARE.md](HARDWARE.md): PC 조사 결과
- [docs/PROTOCOLS.md](docs/PROTOCOLS.md): 프로토콜 근거와 검증 범위

자동 시작은 현재 사용자 전용 `RGB Switch - <SID>` 예약 작업으로 등록하며, 다른 앱의 시작 설정은 변경하지 않습니다. 메모리용 커널 드라이버 서비스는 수동 시작이며, 사용 후 해당 Windows 세션에 로드된 상태로 남을 수 있습니다.

`examples/lighting_test.rs`는 실제 조명을 변경하는 명시적 진단입니다. GPU·메인보드·메모리는 이전 설정을 복원하며, 키보드는 흰색, 마우스는 DPI가 동작하는 기본 모드 ON으로 끝납니다. `cargo run --example lighting_test -- 2`처럼 장치 ID(1 GPU / 2 메인보드 / 3 키보드 / 4 메모리)를 지정할 수 있습니다. 메모리는 관리자 실행이 필요합니다.
