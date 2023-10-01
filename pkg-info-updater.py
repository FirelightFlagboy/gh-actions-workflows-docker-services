from __future__ import annotations

from argparse import ArgumentParser
from pathlib import Path
from operator import attrgetter
from typing import cast, BinaryIO
from dataclasses import dataclass

import itertools
import csv
import subprocess
import json


class EnhancedJSONEncoder(json.JSONEncoder):
    def default(self, o):
        import dataclasses

        if dataclasses.is_dataclass(o):
            return dataclasses.asdict(o)
        return super().default(o)


@dataclass
class PkgInfo:
    repository: str
    latest: str | None
    arch_asset_patterns: dict[str, str]
    versions: dict[str, dict[str, str]]

    @staticmethod
    def from_reader(r: BinaryIO) -> PkgInfo:
        raw_data = json.load(r)

        return PkgInfo(
            repository=raw_data["repository"],
            latest=raw_data.get("latest", None),
            arch_asset_patterns=raw_data["arch_asset_patterns"],
            versions=raw_data.get("versions", {}),
        )

    def get_version(self, version: str) -> dict[str, str]:
        return self.versions.setdefault(version, {})


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument("--file", "-f", default=Path("pkg_info.json"), type=Path)
    parser.add_argument("--repo-type", choices=["github"], default="github")
    parser.add_argument("--tmp-dir", default=Path("/tmp"), type=Path)
    parser.add_argument("--test", action="store_true")

    (FILE, TMP_DIR, REPO_TYPE, TEST) = cast(
        tuple[Path, Path, str],
        attrgetter("file", "tmp_dir", "repo_type", "test")(parser.parse_args()),
    )
    assert REPO_TYPE == "github"
    with FILE.open() as f:
        data = PkgInfo.from_reader(f)

    GH_RELEASE_PREFIX = ["gh", "release", f"--repo={data.repository}"]
    cmd = subprocess.run(
        [*GH_RELEASE_PREFIX, "list", "--exclude-drafts", "--exclude-pre-releases", "--limit=1"],
        stdout=subprocess.PIPE,
        check=True,
    )

    reader = csv.reader(cmd.stdout.decode().splitlines(), delimiter="\t")
    release_name, release_type, release_tag, release_date = next(reader)
    assert release_type.lower() == "latest"
    print(f"Latest release {release_name} ({release_date})")

    data.latest = release_name

    patterns = data.arch_asset_patterns.values()
    subprocess.check_call(
        [
            *GH_RELEASE_PREFIX,
            "download",
            release_tag,
            f"--dir={TMP_DIR}",
            *(["--skip-existing"] if TEST else {}),
            *[f"--pattern={pat}" for pat in patterns],
        ]
    )

    args = itertools.chain(["sha512sum"], *[list(Path(TMP_DIR).glob(pat)) for pat in patterns])
    cmd = subprocess.run(args, check=True, stdout=subprocess.PIPE)

    for line in cmd.stdout.splitlines():
        checksum, file = line.decode().split("  ")
        file = Path(file)
        arch = next(
            arch for arch, pattern in data.arch_asset_patterns.items() if file.match(pattern)
        )
        data.get_version(release_name)[arch] = {"sha512": checksum, "filename": file.name}

    with FILE.open("w") as f:
        json.dump(data, f, indent=2, cls=EnhancedJSONEncoder)
        f.write("\n")
