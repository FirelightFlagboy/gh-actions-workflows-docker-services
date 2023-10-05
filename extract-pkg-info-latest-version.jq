  .name as $name
| .latest_version as $version
| "amd64" as $arch
| .versions[$version][$arch] as $manifest
| $manifest.download_url as $download_url
| $manifest.filename as $filename
| $manifest.digest | ltrimstr("sha512:") as $sha512
| ["name=\($name)", "version=\($version)", "download_url=\($download_url)", "filename=\($filename)", "sha512=\($sha512)"] | join("\n")
