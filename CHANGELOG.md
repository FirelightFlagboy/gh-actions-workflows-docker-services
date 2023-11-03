# GH-actions-workflows-docker-services

## Unreleased

### Docker-build-publish workflow

- Add the following required inputs: ([PR-23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))
  - `pkg-file`: The path to the `pkg-file` (default to `pkg-info.json`).
  - **(BREAKING CHANGE)** `docker-repository`: The repository name to push the image.
- The input `pkg-version` is now optional (will use the latest version defined in `pkg-file`). ([PR-23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))

## Update-pkg-info workflow

- Correct typo on description ([PR-22](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/22))

### Internal change

- Add action `pkg-version-to-use`. ([PR-23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))

  The action will:

  - Take 2 inputs:
    - The desired package version (optional).
    - The package file path.
  - If we don't provide the package version, it will use the latest version defined in the package file.
  - Will ensure the selected version is defined in the package file.
  - Return 2 outputs:
    - The version to use either the version provided or latest.
    - A boolean if the returned version is the latest version.

  ```mermaid
  flowchart TB
    IN_VER[Package version]
    IN_FILE[Package file]
    OUT_VER[version to use]
    OUT_LATEST[version is latest]

    IN_VER & IN_FILE --> ACT
    ACT --> OUT_VER
    ACT --> OUT_LATEST

    subgraph ACT[pkg-version-to-use]
      direction TB
      VER_SEL{version is set ?}
      USE_LATEST[Use latest version]
      VER_EXIST{version exist ?}
      VER_LATEST{version is latest ?}

      VER_SEL -- YES --> VER_EXIST
      VER_SEL -- NO --> USE_LATEST --> VER_EXIST

      VER_EXIST -- NO --> ACT_FAIL[The action fail]
      VER_EXIST -- YES --> VER_LATEST
    end
  ```
