#!/usr/bin/env python3
"""Rebuild the vanilla 647 client with the repository's login and JS5 RSA key and 648's socket reader fix."""

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

ROOT = Path(__file__).resolve().parent.parent
SOURCE = "https://archive.openrs2.org/clients/31063.dat"
SHA256 = "b7d38915a8fb878d2e2762ba63dac195e68fc8a7ecd9dcc1e738e17f875b6086"
MODULI = {"kw.class": 128, "jf.class": 1024}


def patch_modulus(data, digits, modulus):
    pattern = rb"\x01" + re.escape(struct.pack(">H", digits)) + rb"[0-9a-f]{" + str(digits).encode() + rb"}"
    matches = list(re.finditer(pattern, data))
    if len(matches) != 1:
        raise ValueError("Expected exactly one RSA modulus constant")
    match = matches[0]
    replacement = b"\x01" + struct.pack(">H", len(modulus)) + modulus
    return data[:match.start()] + replacement + data[match.end():]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original", type=Path, help="Original JAR; otherwise download the pinned OpenRS2 release")
    parser.add_argument("--public-key", type=Path, default=ROOT / ".keys/public-647.pem")
    parser.add_argument("--output", type=Path, default=ROOT / "public/647/client.jar")
    parser.add_argument("--check", action="store_true", help="Verify the existing output without writing it")
    args = parser.parse_args()

    if args.original:
        original = args.original.read_bytes()
    else:
        with urlopen(SOURCE, timeout=30) as response:
            original = response.read()
    if hashlib.sha256(original).hexdigest() != SHA256:
        raise ValueError("Original JAR does not match the pinned vanilla 647 release")

    key = subprocess.run(
        ["openssl", "rsa", "-pubin", "-in", str(args.public_key), "-modulus", "-text", "-noout"],
        check=True, capture_output=True, text=True,
    ).stdout
    exponent = re.search(r"Exponent: (\d+)", key)
    modulus = re.search(r"^Modulus=([0-9A-Fa-f]+)$", key, re.MULTILINE)
    if not exponent or int(exponent[1]) != 65537 or not modulus:
        raise ValueError("The vanilla client requires an RSA public key with exponent 65537")
    modulus = modulus[1].lower().encode("ascii")

    output = io.BytesIO()
    with ZipFile(io.BytesIO(original)) as source, ZipFile(output, "w") as patched:
        reader, fixed = fix_reader(source)
        for entry in source.infolist():
            data = source.read(entry)
            if entry.filename in MODULI:
                data = patch_modulus(data, MODULI[entry.filename], modulus)
            if entry.filename == reader:
                data = fixed
            patched.writestr(entry, data)

    with ZipFile(io.BytesIO(original)) as source, ZipFile(output) as patched:
        changed = {name for name in source.namelist() if source.read(name) != patched.read(name)}
        if changed != MODULI.keys() | {reader}:
            raise ValueError(f"Unexpected changed entries: {changed}")

    if args.check:
        with ZipFile(args.output) as existing, ZipFile(output) as patched:
            if existing.namelist() != patched.namelist():
                raise ValueError("Client JAR entries differ from the vanilla release")
            for name in patched.namelist():
                if existing.read(name) != patched.read(name):
                    raise ValueError(f"Unexpected client data: {name}")
        print("Verified: only the RSA modulus constants and the socket reader check differ from vanilla 647")
    else:
        args.output.write_bytes(output.getvalue())
        print(f"Wrote {args.output}: only the RSA modulus constants and the socket reader check changed")


if __name__ == "__main__":
    main()
