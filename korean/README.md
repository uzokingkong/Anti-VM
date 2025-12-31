[🇺🇸 English Version](../README.md)

# Anti-VM: Hardware Encoder 기반 Sandbox 회피 기법

![배너](../images/social-preview.png)

![토픽](https://img.shields.io/badge/topics-anti--vm%20%7C%20bypass--sandbox%20%7C%20bypass--vm%20%7C%20windows-blue)

> ⚠️ **면책 조항**
> 본 문서는 교육 및 보안 연구 목적으로만 작성되었습니다. 본문에 기술된 기법을 허가받지 않은 시스템에서 악용하는 것은 불법이며 발생하는 모든 책임은 사용자에게 있습니다.

## 개요: 하드웨어 기능 격차를 이용한 신규 Anti-VM 접근

최근 화면 녹화 프로그램(Screen Recorder)을 개발하던 중 흥미로운 현상을 발견했습니다. 
Windows Media Foundation(WMF)의 하드웨어 비디오 인코더 기능이 가상 머신(VM) 환경에서는 정상적으로 동작하지 않는다는 점이었습니다.

해당 기능은 실제 사용자 PC 환경에서는 거의 필수적으로 제공되는 반면, 
많은 VM이나 자동화 분석용 샌드박스 환경에서는 지원되지 않는 경우가 많았습니다. 
즉, 실제 사용자 환경과 분석 환경 사이에 명확한 기능적 격차가 존재한다는 사실을 확인할 수 있었습니다.

이러한 차이를 악성코드의 분석 회피 기법에 전략적으로 활용할 수 있겠다는 아이디어를 떠올렸고, 
이를 검증하기 위해 간단한 PoC 로더를 구현했습니다. 
본 문서는 제가 발견한 이 기법의 기술적 원리와 실제 테스트 결과를 정리한 것입니다.

---

## 기술적 원리: 기능 수행 여부 기반 환경 판별

이 로더는 단순한 환경 문자열 비교나 레지스트리 검사 대신 “실제로 하드웨어 기능을 수행할 수 있는가?”라는 질문에 초점을 맞춥니다. 이를 위해 두 단계의 검증 절차를 사용합니다.

---

### Step 1: 하드웨어 비디오 인코더 열거 (`MFTEnumEx`)

첫 번째 단계는 시스템에 **하드웨어 가속 비디오 인코더(MFT)** 가 등록되어 있는지를 확인하는 것입니다.

| API | 목적 | 핵심 플래그 |
| :--- | :--- | :--- |
| `MFTEnumEx` | 미디어 변환 필터(MFT) 열거 | `MFT_ENUM_FLAG_HARDWARE` |

*   **일반적인 실제 PC**: GPU가 존재하는 환경에서는 하드웨어 MFT가 등록되어 있으며, 열거 결과는 **0보다 큼**
*   **VM / Sandbox 환경**: 대부분의 가상 환경은 하드웨어 인코더를 노출하지 않기 때문에, 결과가 **0**으로 반환됨 → 이 경우 로더는 즉시 종료

---

### Step 2: 기능 수행 기반 검증 (`MFCreateSinkWriterFromURL`)

단순히 API 결과만 속이는 환경을 방지하기 위해, 두 번째 단계에서는 **실제 하드웨어 인코딩 파이프라인 초기화**를 시도합니다.

검증 흐름은 다음과 같습니다.

1.  `MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS` 속성을 설정하여 하드웨어 가속을 강제
2.  `SinkWriter` 생성
3.  `BeginWriting()` 호출을 통해 인코딩 파이프라인 시작 시도

하드웨어 MFT가 존재하는 것처럼 보이더라도 **실제 드라이버나 인코더가 없으면 이 단계에서 반드시 실패**하게 됩니다.

이로써 단순한 API 후킹이나 더미 값 반환만으로는 우회하기 어려운 **기능적 검증**이 완성됩니다.

---

## 로더 빌드 방법

이 레포지토리에는 완전히 작동하는 스텔스 PE 로더가 포함되어 있으며 다음 기능을 제공합니다:

### 주요 기능

- **문자열 난독화**: `obfstr`을 사용한 컴파일 타임 문자열 암호화
- **VM 감지**: Media Foundation API를 통한 하드웨어 기반 가상 머신 감지
- **XOR 암호화**: XOR 암호를 사용한 페이로드 난독화
- **랜덤 시드**: 빌드마다 고유한 난독화 시드 생성
- **정적 링크**: 외부 DLL 의존성 없음 (VCRUNTIME140.dll 포함)
- **최적화**: LTO 활성화, 크기 최적화, 심볼 제거

### 빌드 요구사항

- **Rust** (최신 stable 버전)
- **Windows** (x64)
- **MSVC** 빌드 도구

### 빠른 시작

#### 1. 페이로드 배치

프로젝트 루트에 실행 파일을 `payload.exe`로 복사:

```powershell
copy your_program.exe payload.exe
```

#### 2. 빌드

```powershell
cargo build --release
```

#### 3. 로더 사용

최종 로더는 다음 위치에 생성됩니다:
```
target\x86_64-pc-windows-msvc\release\ANTI-VM-loader.exe
```

### 프로젝트 구조

```
ANTI-VM/
├── .cargo/
│   └── config.toml          # 정적 CRT 링크 설정
├── src/
│   ├── main.rs              # 로더 로직
│   └── chunks.rs            # 자동 생성 (커밋하지 말 것)
├── test_payload/            # 예제 페이로드
│   ├── src/
│   │   └── main.rs
│   └── Cargo.toml
├── images/                  # 스크린샷
├── korean/                  # 한국어 문서
├── build.rs                 # 페이로드 난독화 스크립트
├── Cargo.toml               # 의존성 및 최적화 설정
├── .gitignore
├── LICENSE
└── README.md
```

### 설정

#### 릴리스 프로필 (Cargo.toml)

```toml
[profile.release]
opt-level = "z"          # 크기 최적화
lto = true               # 링크 타임 최적화
codegen-units = 1        # 단일 코드 생성 유닛
panic = "abort"          # 패닉 정보 제거
strip = true             # 디버그 심볼 제거
```

#### 정적 링크 (.cargo/config.toml)

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

### 예제: test_payload

레포지토리에는 `helloworld.txt`를 생성하는 예제 페이로드가 포함되어 있습니다:

```rust
use std::fs::File;
use std::io::Write;

fn main() {
    let mut file = File::create("helloworld.txt").unwrap();
    file.write_all(b"Hello from stealth loader!").unwrap();
}
```

빌드 및 테스트:

```powershell
cd test_payload
cargo build --release
copy target\x86_64-pc-windows-msvc\release\test_payload.exe ..\payload.exe
cd ..
cargo build --release
```

### 중요 사항

- `payload.exe`를 변경할 때마다 `cargo clean && cargo build --release`로 재빌드하여 새로운 랜덤 시드를 생성해야 합니다
- VM 환경에서는 페이로드를 실행하지 않고 **즉시 종료**됩니다
- 바이너리 크기: 약 500 KB (모든 최적화 적용 시)

---

## PoC 테스트 결과

기법의 실효성을 검증하기 위해, 실제 악성코드 페이로드를 포함한 로더를 제작한 후 **Any.Run을 포함한 총 4개의 자동화 분석 샌드박스**에 업로드하여 테스트를 진행했습니다.

### 테스트 결과 요약

| 샌드박스 | 결과 | 설명 |
| :--- | :--- | :--- |
| Any.Run | **우회 성공** | 로더가 VM 환경으로 판단 후 즉시 종료 |
| 플랫폼 B | **우회 성공** | 페이로드 미실행 |
| 플랫폼 C | **우회 성공** | 악성 행위 로그 없음 |
| 플랫폼 D | **우회 성공** | 분석 실패 |

모든 테스트 환경에서 로더는 분석 환경을 VM으로 인식하고 조용히 종료되었으며 의도한 페이로드는 실행되지 않았습니다. 이는 **현재 주류 자동 분석 시스템이 해당 기법에 취약함**을 보여줍니다.

---

### 테스트 결과 스크린샷

> 📌 아래는 각 샌드박스에서 로더가 정상적으로 종료된 화면 예시입니다.

*   **Any.Run 결과**
    ![Any.Run Result](../images/2025-12-30_211642.png)

*   **플랫폼 B**
    ![Platform B](../images/2025-12-30_211634.png)

*   **플랫폼 C**
    ![Platform C](../images/2025-12-30_211608.png)

---

## 위협 모델 및 시사점

### 1. 하드웨어 격차(Hardware Gap)의 악용

이 기법은 일반 사용자 환경과 분석 서버 간의 **현실적인 하드웨어 차이**를 정확히 활용합니다.

*   **일반 사용자 PC**: 영상 재생·녹화를 위해 GPU 및 하드웨어 인코더가 기본적으로 존재
*   **분석 서버 / 샌드박스**: 비용 효율성 및 밀도 문제로 그래픽 기능이 최소화됨

이로 인해 정보 탈취형 악성코드나 뱅킹 트로이목마가 **보안 연구 환경을 회피하고 실제 사용자만 선택적으로 공격**하는 데 매우 적합한 조건이 됩니다.

---

### 2. 신규성과 높은 회피율

*   **접근 방식의 차별성**: 기존 Anti-VM 기법(CPUID, BIOS 문자열, 레지스트리 검사 등)과 달리 WMF의 멀티미디어 서브시스템을 활용한 방식은 아직 널리 알려지지 않음
*   **높은 회피율**: 복잡한 미디어 파이프라인을 완전히 에뮬레이션하는 샌드박스는 드뭄

---

### 3. Rust 기반 로더의 분석 난이도

로더는 **Rust 언어**로 작성되었습니다. Rust 특유의 바이너리 구조와 컴파일러 최적화는 전통적인 C/C++ 기반 정적·동적 분석 도구의 효율을 저하시켜, 분석 난이도를 한층 높입니다.

---

## 🛑 한계점 및 향후 무력화 가능성

이 기법 역시 만능은 아니며, 다음과 같은 한계가 존재합니다.

### 1. Windows 10 이상 요구

이 기법은 **Windows Media Foundation Transform (MFT)** API를 사용하므로 **Windows 10 이상**에서만 정상적으로 동작합니다. Windows 7/8에서는 MFT 열거 결과가 다를 수 있어 오작동할 가능성이 있습니다.

### 2. 실제 서버 환경 오탐지

GPU가 없는 **물리 서버(DC, 파일 서버 등)** 는 VM으로 오인될 수 있습니다. 따라서 서버 인프라를 주요 공격 대상으로 삼는 시나리오에는 적합하지 않습니다.

---

### 마지막

이 문서는 제가 직접 발견하고 구현한 **하드웨어 기능 수행 여부를 기반으로 한 신규 Anti-VM 기법**에 대한 기술적 분석과 제언입니다.

---

**⚠️ 알림**: 이 프로젝트는 교육 및 보안 연구 목적으로만 제공됩니다. 항상 책임 있는 공개 및 윤리적 지침을 따르십시오.


