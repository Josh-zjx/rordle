"""Package tested web artifacts and assemble versioned GitHub Pages sites."""

import argparse
import hashlib
import json
import re
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

WEB_FILES = ("index.html", "pkg/rordle.js", "pkg/rordle_bg.wasm")
VERSION = re.compile(r"v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def provenance(sha, run_id):
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ValueError("source SHA must be a full Git commit SHA")
    if not re.fullmatch(r"[1-9][0-9]*", str(run_id)):
        raise ValueError("CI run ID must be a positive integer")


def bundle(web, output, sha, run_id):
    provenance(sha, run_id)
    files = {name: (web / name).read_bytes() for name in WEB_FILES}
    if not files["pkg/rordle_bg.wasm"].startswith(b"\0asm\x01\0\0\0"):
        raise ValueError("invalid WebAssembly header")
    manifest = {
        "schema": 1,
        "source_sha": sha,
        "ci_run_id": str(run_id),
        "files": {name: digest(data) for name, data in files.items()},
    }
    files["build.json"] = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    output.mkdir(parents=True, exist_ok=True)
    archive = output / "rordle-web.zip"
    with ZipFile(archive, "w", compression=ZIP_DEFLATED, compresslevel=9) as zipped:
        for name, data in sorted(files.items()):
            info = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            zipped.writestr(info, data, compresslevel=9)
    (output / "SHA256SUMS").write_text(f"{digest(archive.read_bytes())}  rordle-web.zip\n")


def read_bundle(archive, sha=None, run_id=None):
    with ZipFile(archive) as zipped:
        names = zipped.namelist()
        if len(names) != 4 or set(names) != {*WEB_FILES, "build.json"}:
            raise ValueError(f"unexpected bundle contents: {archive}")
        if sum(info.file_size for info in zipped.infolist()) > 64 * 1024 * 1024:
            raise ValueError(f"bundle exceeds 64 MiB: {archive}")
        files = {name: zipped.read(name) for name in names}
    manifest = json.loads(files["build.json"])
    provenance(manifest["source_sha"], manifest["ci_run_id"])
    if manifest.get("schema") != 1 or manifest["files"] != {
        name: digest(files[name]) for name in WEB_FILES
    }:
        raise ValueError(f"bundle checksums do not match: {archive}")
    if not files["pkg/rordle_bg.wasm"].startswith(b"\0asm\x01\0\0\0"):
        raise ValueError(f"invalid WebAssembly header: {archive}")
    if sha is not None and manifest["source_sha"] != sha:
        raise ValueError("bundle source SHA does not match the approved CI run")
    if run_id is not None and manifest["ci_run_id"] != str(run_id):
        raise ValueError("bundle CI run ID does not match the approved CI run")
    return files, manifest


def write_site(destination, files):
    for name, data in files.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


def pages(archive, output, history, version, sha, run_id):
    if not VERSION.fullmatch(version):
        raise ValueError("release version must be vMAJOR.MINOR.PATCH, e.g. v0.1.0")
    if output.exists() and any(output.iterdir()):
        raise ValueError("Pages output directory must be empty")
    current, manifest = read_bundle(archive, sha, run_id)
    releases = {}
    for directory in sorted(history.iterdir()) if history.exists() else ():
        if not directory.is_dir() or not VERSION.fullmatch(directory.name):
            raise ValueError(f"unexpected release history entry: {directory}")
        if directory.name == version:
            raise ValueError(f"release {version} already exists; use a new version")
        files, previous = read_bundle(directory / "rordle-web.zip")
        releases[directory.name] = (files, previous)
    releases[version] = (current, manifest)
    write_site(output, current)
    for tag, (files, _) in releases.items():
        write_site(output / "releases" / tag, files)
    index = {
        "latest": version,
        "releases": {tag: item[1]["source_sha"] for tag, item in sorted(releases.items())},
    }
    (output / "releases.json").write_text(json.dumps(index, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    pack = commands.add_parser("bundle")
    pack.add_argument("--web", type=Path, default=Path("web"))
    pack.add_argument("--output", type=Path, default=Path("dist"))
    deploy = commands.add_parser("pages")
    deploy.add_argument("--bundle", type=Path, required=True)
    deploy.add_argument("--history", type=Path, default=Path("history"))
    deploy.add_argument("--output", type=Path, default=Path("_site"))
    deploy.add_argument("--version", required=True)
    for command in (pack, deploy):
        command.add_argument("--sha", required=True)
        command.add_argument("--run-id", required=True)
    args = parser.parse_args()
    try:
        if args.command == "bundle":
            bundle(args.web, args.output, args.sha, args.run_id)
        else:
            pages(args.bundle, args.output, args.history, args.version, args.sha, args.run_id)
    except (ValueError, KeyError, OSError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
