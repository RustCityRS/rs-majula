#!/usr/bin/env python3
"""Rebuild a revision's original Java client with the repository's login and JS5 RSA key
(and, for 647, 648's socket reader fix), changing nothing else."""

import argparse
import hashlib
import io
import re
import struct
import subprocess
import sys
from pathlib import Path
from urllib.request import urlopen
from zipfile import ZipFile

sys.path.insert(0, str(Path(__file__).resolve().parent))
from patch_client_reader import fix_reader  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]

# Per revision: where the original JAR comes from (None: pass --original), its SHA-256, the
# modulus constants by class as (digits, leading digits that pick it out), and whether the
# socket reader fix applies.
REVS = {
    "647": {
        "source": "https://archive.openrs2.org/clients/31063.dat",
        "sha256": "b7d38915a8fb878d2e2762ba63dac195e68fc8a7ecd9dcc1e738e17f875b6086",
        "moduli": {"kw.class": [(128, b"")], "jf.class": [(1024, b"")]},
        "fix_reader": True,
    },
    # ap holds a second, unused 128-digit constant, so the login modulus is picked by prefix.
    "742": {
        "source": None,
        "sha256": "5880bbdd9287ef530dca72f33443acd43314e73aab4b6c77592fd361a2fe82d6",
        "moduli": {"ap.class": [(1024, b""), (128, b"b53461e2")]},
        "fix_reader": False,
    },
}


def patch_modulus(data, digits, prefix, modulus):
    pattern = (rb"\x01" + re.escape(struct.pack(">H", digits)) + re.escape(prefix)
               + rb"[0-9a-f]{" + str(digits - len(prefix)).encode() + rb"}")
    matches = list(re.finditer(pattern, data))
    if len(matches) != 1:
        raise ValueError("Expected exactly one RSA modulus constant")
    match = matches[0]
    replacement = b"\x01" + struct.pack(">H", len(modulus)) + modulus
    return data[:match.start()] + replacement + data[match.end():]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rev", required=True, choices=sorted(REVS))
    parser.add_argument("--original", type=Path, help="Original JAR; otherwise download the pinned OpenRS2 release")
    parser.add_argument("--public-key", type=Path, default=ROOT / ".keys/js5/public.pem")
    parser.add_argument("--output", type=Path, help="Defaults to public/<rev>/client.jar")
    parser.add_argument("--check", action="store_true", help="Verify the existing output without writing it")
    args = parser.parse_args()
    rev = REVS[args.rev]
    output_path = args.output or ROOT / "public" / args.rev / "client.jar"

    if args.original:
        original = args.original.read_bytes()
    elif rev["source"]:
        with urlopen(rev["source"], timeout=30) as response:
            original = response.read()
    else:
        parser.error(f"--original is required for {args.rev}: there is no pinned download")
    if hashlib.sha256(original).hexdigest() != rev["sha256"]:
        raise ValueError(f"Original JAR does not match the pinned {args.rev} client")

    key = subprocess.run(
        ["openssl", "rsa", "-pubin", "-in", str(args.public_key), "-modulus", "-text", "-noout"],
        check=True, capture_output=True, text=True,
    ).stdout
    exponent = re.search(r"Exponent: (\d+)", key)
    modulus = re.search(r"^Modulus=([0-9A-Fa-f]+)$", key, re.MULTILINE)
    if not exponent or int(exponent[1]) != 65537 or not modulus:
        raise ValueError("The client requires an RSA public key with exponent 65537")
    modulus = modulus[1].lower().encode("ascii")

    output = io.BytesIO()
    with ZipFile(io.BytesIO(original)) as source, ZipFile(output, "w") as patched:
        reader, fixed = fix_reader(source) if rev["fix_reader"] else (None, None)
        for entry in source.infolist():
            data = source.read(entry)
            for digits, prefix in rev["moduli"].get(entry.filename, []):
                data = patch_modulus(data, digits, prefix, modulus)
            if entry.filename == reader:
                data = fixed
            patched.writestr(entry, data)

    expected = set(rev["moduli"]) | ({reader} if reader else set())
    with ZipFile(io.BytesIO(original)) as source, ZipFile(output) as patched:
        changed = {name for name in source.namelist() if source.read(name) != patched.read(name)}
        if changed != expected:
            raise ValueError(f"Unexpected changed entries: {changed}")

    what = "the RSA modulus constants" + (" and the socket reader check" if reader else "")
    if args.check:
        with ZipFile(output_path) as existing, ZipFile(output) as patched:
            if existing.namelist() != patched.namelist():
                raise ValueError(f"Client JAR entries differ from the original {args.rev} client")
            for name in patched.namelist():
                if existing.read(name) != patched.read(name):
                    raise ValueError(f"Unexpected client data: {name}")
        print(f"Verified: only {what} differ from the original {args.rev} client")
    else:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_bytes(output.getvalue())
        print(f"Wrote {output_path}: only {what} changed")


if __name__ == "__main__":
    main()
