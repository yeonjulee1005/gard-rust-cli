변경사항을 커밋해줘.

## Preflight

- 현재 브랜치가 `develop` 또는 `main`이면 멈추고 feature 브랜치 생성을 안내한다.

## 실행 절차

1. `cargo fmt --check` 실행
   - 실패하면 `cargo fmt`로 자동 수정 후 재확인
2. `cargo clippy -- -D warnings` 실행
   - 실패 시 `git diff --name-only`로 변경된 파일 목록 확인
   - 변경된 파일에서 발생한 경고/에러만 수정, 수정 후 재실행
3. `cargo test` 실행
   - 실패 시 변경 파일에서 발생한 테스트 에러만 수정 후 재실행
   - `#[ignore]` 테스트는 실행하지 않음 (네트워크 테스트 제외)
4. 세 가지 모두 통과하면 `.claude/rules/commit-convention.md`의 컨벤션에 맞게 커밋
5. 변경사항이 여러 목적(기능 추가, 버그 수정, 리팩토링 등)에 걸쳐 있으면 관련 파일끼리 묶어 분리 커밋
6. `.env`, credentials, `.cargo/credentials.toml` 등 시크릿 파일은 커밋하지 않는다

## 커밋 메시지

- 영어로 작성 (`.claude/rules/commit-convention.md` 참고)
- HEREDOC으로 전달
- 사용자가 명시적으로 요청하지 않으면 amend·force push 금지
