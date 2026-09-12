# Local hardware inventory

Checked 2026-09-12 through Windows CIM/PnP, exact-device USB traffic, and controller readback. Serial numbers omitted.

| Device | Exact model / transport | Verified result |
| --- | --- | --- |
| Mouse | CORSAIR HARPOON RGB WIRELESS; USB 1B1C:1B5E, interface 1, FF42:0001 | Wired OFF / white ON acknowledged; software render mode read back |
| GPU | Palit RTX 3070; PCI 10DE:2484, subsystem 1569:2484; NVAPI I²C | OFF / white ON register readback; original six registers restored |
| Motherboard | MSI MAG B860M MORTAR WIFI MS-7E40; USB 0DB0:0076, interface 0, FF00:0001; firmware 2.0 | OFF / ON global switch readback; all 290 effect bytes unchanged; original ON restored |
| Keyboard | GANSS GS3087T; USB 05AC:0256, interface 0, 0001:0006 | OFF: all 144 color slots zero. White ON: 88 slots FD/FD/FD, 56 zero. No flash save |
| Memory | 2 × 64 GB CORSAIR CMH128GX5M2B6400C42; SMBus 0x19 / 0x1B | Both controllers 1B1C:0901, protocol 4, firmware 0.5.6; OFF / ON effect readback and original 20-byte effects restored |
| Fans / case lighting | MSI motherboard ordinary Gen1 ARGB path; individual fan/case models not exposed by Windows | User confirmed all case fan LEDs off after explicit Gen1 port initialization and native Off; prior global OFF and color writes without initialization failed |

Keyboard interface 1 / FFFF:0001 is an input-only collection, **not** its RGB feature endpoint. The verified endpoint has a 95-byte HID descriptor with an unnumbered 64-byte feature report. The first test stayed pink because the active effect byte was missing; after correcting byte 12, actual color readback verified white.

Memory uses the existing installed Corsair low-level driver with administrator rights. The actual Intel 8086:7F23 SMBus I/O resource is 0x4000–0x401F. Access is restricted to this PC, this DLL version, the two known RGB-controller addresses, and lighting-related registers. SPD addresses and firmware commands are rejected. The first write test hit temporary NACKs during effect application; bounded status-read polling resolved it, and both modules were restored to the pre-test rainbow effect with brightness 255.

The SLIPSTREAM receiver 1B1C:1BDC is not a separate RGB device. Intel integrated graphics 8086:7D67 and Razer Seiren Mini 1532:0531 have no established controllable RGB here. No separate USB fan RGB controller was found. Ordinary header-connected fans are not individually enumerated.

The exact-model GANSS utility launched for protocol capture was terminated afterward, including its tray process and capture helper. Existing iCUE / MSI services were not uninstalled or disabled. Other lighting applications can overwrite settings; protocol checks do not establish optical illumination.


## 2026-09-12 구현 업데이트

현재 EXE는 메모리 DLL/SYS를 내장하고 앱 전용 수동 시작 드라이버 서비스로 접근합니다. 기존 Corsair 설치 폴더를 런타임 의존 경로로 쓰지 않습니다. GPU·MSI 헤더·GS3087T·DDR5 두 개는 단색/숨쉬기/색상 순환의 직접 제어와 응답을 확인했습니다. 마우스는 소프트웨어 2-LED 프레임을 제거하고 기본 하드웨어 모드+밝기 방식으로 변경했습니다. 사용자가 DPI 버튼에 따라 표시등 색상이 바뀌는 것을 확인했습니다. 자세한 현행 프로토콜과 검증은 docs/PROTOCOLS.md 마지막 절을 따릅니다.

## 2026-09-13 마우스 단색·DPI 설정

이전 마우스 효과 제외 방식을 대체했습니다. 유선 인터페이스 1에서 로고와 DPI 표시등의 RGB를 별도로 보내고, 인터페이스 2 / FF42:0002에서 DPI 버튼의 눌림 전환을 처리합니다. 사용자가 분홍색 로고 유지와 버튼별 표시등·감도 변화를 확인했습니다. 앱 저장 DPI는 200~10000, 200 단위입니다. 실기기에서 1700은 거부, 200/1200/1800/5200/10000은 현재 감도 속성 20 재조회까지 일치했습니다. 하드웨어에 저장된 5단계 값·색상·활성 마스크는 수정하지 않습니다. 사용자 지정 값은 앱 실행 중에만 적용하며 종료 후 기존 프리셋과 모드 1 복원을 검증했습니다. iCUE 프로세스는 진단 중 종료했으며 서비스와 시작 설정은 변경하지 않았습니다. 본체 팬·다른 기기는 이 마우스 진단에서 변경하지 않았습니다.
