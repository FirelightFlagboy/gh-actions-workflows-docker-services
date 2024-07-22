<!-- markdownlint-configure-file { "first-line-heading": { "level": 3 } } -->
### Schema change

- Add optional `strip_v_prefix:bool` field ([#54](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/54))

  Setting this field to `true` will strip the v prefix from versions.

- Add optional `allow_prerelease:bool` field ([#55](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/55))

  Setting it to `true` will allow `github-release` mode to use release mark as `prerelease`.
  For `bash_command` and `jq_script` the env variable `ALLOW_PRERELEASE=1` is exported.

### Fixes

- Fix missing artifact when shared between multiple architectures ([#53](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/53))

### Others

- Bump rust dependencies ([#52](https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/pull/52))
