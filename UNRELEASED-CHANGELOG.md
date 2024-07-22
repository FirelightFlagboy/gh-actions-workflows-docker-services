<!-- markdownlint-configure-file { "first-line-heading": { "level": 3 } } -->
- Harden download of `pkg-info-updater`

  The step just after it complain about missing the exe, but no error where raise during the download step.

:warning: Previous `2.1.0` was compiled on nix, resulting in a binary that do not work on other system.
