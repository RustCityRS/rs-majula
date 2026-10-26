#!/usr/bin/env python3
"""Runs patch.py on a scratch copy of the repo's web clients and key, as a
deploy would, and checks what comes out:

- every client gets each patch exactly once and still parses (node --check);
- running it again leaves the patches as they are and only swaps the key;
- a client it can't patch stops the run with nothing changed.

Run from the repo root: python3 deploy/test_patch.py
Needs python3, openssl and node on the path.
"""

import glob
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.dont_write_bytecode = True
sys.path.insert(0, HERE)
import patch  # noqa: E402

PATCH = os.path.join(HERE, "patch.py")
MARKERS = (
    ("login uid", patch.PATCHED_UID),
    ("fullscreen check", patch.LETTERBOXED),
    ("update countdown end", "this.rebootAt=Date.now()+20*this."),
    ("update countdown tick", "(this.rebootAt-Date.now())/20"),
)


def fail(msg):
    sys.exit(f"FAIL: {msg}")


def stage(root):
    """A release-shaped copy of the key and every client.js under `root`."""
    shutil.copytree(".keys", os.path.join(root, ".keys"))
    for path in sorted(glob.glob("public/*/client.js")):
        dest = os.path.join(root, path)
        os.makedirs(os.path.dirname(dest))
        shutil.copyfile(path, dest)


def snapshot(root):
    files = {}
    for path in glob.glob(os.path.join(root, "**", "*"), recursive=True):
        if os.path.isfile(path):
            with open(path, "rb") as f:
                files[os.path.relpath(path, root)] = f.read()
    return files


def run_patch(root):
    return subprocess.run([sys.executable, PATCH], cwd=root, capture_output=True, text=True)


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def check_clients(root, old_key):
    clients = sorted(glob.glob(os.path.join(root, "public", "*", "client.js")))
    if not clients:
        fail("no clients staged")
    for path in clients:
        js = read(path)
        if f'BigInt("{old_key}")' in js:
            fail(f"{path}: still has the old key")
        for what, marker in MARKERS:
            if js.count(marker) != 1:
                fail(f"{path}: expected its {what} once, found it {js.count(marker)} times")
        node = subprocess.run(["node", "--check", path], capture_output=True, text=True)
        if node.returncode != 0:
            fail(f"{path}: patched client doesn't parse:\n{node.stderr}")
    return clients


def main():
    if not os.path.isdir(".keys") or not glob.glob("public/*/client.js"):
        fail("run from the repo root: need .keys/ and public/<rev>/client.js")

    with tempfile.TemporaryDirectory() as root:
        stage(root)
        key = os.path.join(root, patch.PRIVATE)

        before, _ = patch.rsa_numbers(key)
        result = run_patch(root)
        if result.returncode != 0:
            fail(f"first run failed:\n{result.stdout}{result.stderr}")
        clients = check_clients(root, before)
        first = {path: read(path) for path in clients}

        again, _ = patch.rsa_numbers(key)
        result = run_patch(root)
        if result.returncode != 0:
            fail(f"second run failed:\n{result.stdout}{result.stderr}")
        check_clients(root, again)
        after, _ = patch.rsa_numbers(key)
        for path in clients:
            # Only the key may differ between the two runs.
            if read(path).replace(str(after), str(again)) != first[path]:
                fail(f"{path}: a second run changed more than the key")

    for client in sorted(glob.glob("public/*/client.js")):
        with tempfile.TemporaryDirectory() as root:
            stage(root)
            broken = os.path.join(root, client)
            js = read(broken)
            # The countdown drawn from some other variable: a client the patch no longer knows.
            js = js.replace('"System update in: "', '"System update at: "')
            with open(broken, "w", encoding="utf-8", newline="") as f:
                f.write(js)
            untouched = snapshot(root)
            result = run_patch(root)
            if result.returncode == 0:
                fail(f"{client}: a client without its countdown still patched")
            if "system update countdown" not in result.stderr:
                fail(f"{client}: stopped for another reason:\n{result.stderr}")
            if snapshot(root) != untouched:
                fail(f"{client}: a stopped run changed files")

    print(f"OK: patched {len(clients)} clients twice; a client it can't patch stops the run unchanged.")


if __name__ == "__main__":
    main()
