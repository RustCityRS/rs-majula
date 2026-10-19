#!/usr/bin/env python3
"""Replaces the login RSA key with a freshly generated one.

Run on the server from the extracted release directory (the one holding
.keys/ and public/), before starting the world:

    python3 rotate-rsa.py

It generates a new key with openssl, writes it to .keys/private.pem (and
.keys/public.pem), and swaps the old key's modulus and exponent for the new
ones in every public/<rev>/client.js, so the web client encrypts logins to the
new key. The server reads both at startup, so restart the world afterwards.

The Java client (client.jar) keeps the old key and can't log in after this.
"""

import base64
import glob
import os
import subprocess
import sys

# The client sends the encrypted block, and the login packet holding it, with
# one-byte lengths: the packet (block + ~41 bytes) must stay within 255 bytes,
# which caps the modulus near 1700 bits. 2048 doesn't fit.
KEY_BITS = 1024
PRIVATE = ".keys/private.pem"
PUBLIC = ".keys/public.pem"


def der_read(data, i):
    """Reads one DER TLV at `i`: (tag, value start, value end)."""
    tag = data[i]
    length = data[i + 1]
    i += 2
    if length & 0x80:
        n = length & 0x7F
        length = int.from_bytes(data[i:i + n], "big")
        i += n
    return tag, i, i + length


def rsa_numbers(pem_path):
    """The (modulus, public exponent) of a PKCS#8 RSA private key."""
    with open(pem_path, encoding="ascii") as f:
        lines = f.read().splitlines()
    if not lines or lines[0].strip() != "-----BEGIN PRIVATE KEY-----":
        sys.exit(f"{pem_path}: expected a PKCS#8 'BEGIN PRIVATE KEY' file")
    der = base64.b64decode("".join(line.strip() for line in lines if not line.startswith("-----")))
    _, i, _ = der_read(der, 0)            # PrivateKeyInfo SEQUENCE
    _, _, i = der_read(der, i)            # version
    _, _, i = der_read(der, i)            # algorithm
    tag, i, _ = der_read(der, i)          # privateKey OCTET STRING
    if tag != 0x04:
        sys.exit(f"{pem_path}: not a PKCS#8 private key")
    _, i, _ = der_read(der, i)            # RSAPrivateKey SEQUENCE
    _, _, i = der_read(der, i)            # version
    _, start, end = der_read(der, i)      # modulus
    n = int.from_bytes(der[start:end], "big")
    _, start, end = der_read(der, end)    # publicExponent
    e = int.from_bytes(der[start:end], "big")
    return n, e


def openssl(*args):
    result = subprocess.run(["openssl", *args], capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"openssl {' '.join(args)} failed:\n{result.stderr}")


def write_atomic(path, data):
    tmp = path + ".new"
    with open(tmp, "w", encoding="utf-8", newline="") as f:
        f.write(data)
    return tmp


def main():
    clients = sorted(glob.glob("public/*/client.js"))
    if not os.path.exists(PRIVATE) or not clients:
        sys.exit(f"run from the release directory: need {PRIVATE} and public/<rev>/client.js")

    old_n, old_e = rsa_numbers(PRIVATE)

    new_key = PRIVATE + ".new"
    openssl("genpkey", "-algorithm", "RSA", "-pkeyopt", f"rsa_keygen_bits:{KEY_BITS}", "-out", new_key)
    new_n, new_e = rsa_numbers(new_key)

    # Patch every client in memory first: nothing is replaced unless all match.
    staged = [new_key]
    for path in clients:
        with open(path, encoding="utf-8") as f:
            js = f.read()
        for old, new in ((old_n, new_n), (old_e, new_e)):
            literal = f'BigInt("{old}")'
            count = js.count(literal)
            if count != 1:
                for tmp in staged:
                    os.remove(tmp)
                sys.exit(f"{path}: expected the current key once, found it {count} times "
                         f"(does {PRIVATE} match this client?)")
            js = js.replace(literal, f'BigInt("{new}")')
        staged.append(write_atomic(path, js))

    for tmp in staged:
        os.replace(tmp, tmp[: -len(".new")])
    openssl("pkey", "-in", PRIVATE, "-pubout", "-out", PUBLIC)
    os.chmod(PRIVATE, 0o600)

    print(f"New {KEY_BITS}-bit login key in {PRIVATE}; patched {', '.join(clients)}.")
    print("Restart the world to load it.")


if __name__ == "__main__":
    main()
