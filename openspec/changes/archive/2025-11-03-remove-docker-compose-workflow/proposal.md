## Why
- The repository only builds a single Docker image, yet contributor docs instruct using `docker compose` to validate it.
- Maintaining a compose definition for one image adds redundant configuration and confuses contributors who can rely on plain Docker commands.

## What Changes
- Retire the `docker-compose.yml` workflow and document direct `docker build`/`docker run` commands for packaging validation.
- Update contributor guidance in `README.md` and `AGENTS.md` to reflect the single-image Docker workflow.
- Capture the documentation expectations in the `repo-guidelines` specification so the guidance stays aligned.

## Impact
- Contributors follow a single, consistent Docker workflow without needing Docker Compose.
- Maintenance overhead decreases by removing redundant tooling; runtime behavior of the exporter remains unchanged.
