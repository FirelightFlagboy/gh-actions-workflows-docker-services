<!-- markdownlint-configure-file { "first-line-heading": { "level": 3 } } -->
### Docker-build-publish workflow

#### Breaking Change

- Add new required inputs `docker-repository`: The repository name to push the image to. ([#23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))
- Remove inputs `tags` ([#27](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))

#### Other Change

- Add the inputs `pkg-file`: The path to the `pkg-file` (default to `pkg-info.json`). ([#23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))
- The input `pkg-version` is now optional (will use the latest version defined in `pkg-file`). ([#23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))

### Update-pkg-info workflow

- Correct typo on description ([#22](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/22))
- Fix failure when branch already exit ([#17](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/17))

  Now if the branch already exist, it will be reset to the change made by this workflow (we `git push --force` the change).

### Internal change

- Add action `called-workflow-ref`. ([#21](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/21))

  This action is used when a called workflow need to know its ref used.

  The action will:

  - Take 4 inputs:
    - The source workflow ref (default to `github.workflow_ref`).
    - The source repository (default to `github.repository`).
    - The called workflow repository.
    - The called workflow path.
  - Will look in the source workflow for the pattern of the called workflow to extract its **FIRST** reference.
  - Output the called workflow ref specified in the source workflow.

- Add action `pkg-version-to-use`. ([#23](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/23))

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

- Fix `can't find 'action.yml'` ([#28](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/28) & [#29](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/29))

  The reusable workflows `docker-build-publish` & `update-pkg-info` used local action that doesn't exist when called from an external repository
  Because the action aren't present.

- Fix action cannot find utility script when used on a external repo ([#31](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/31)).
