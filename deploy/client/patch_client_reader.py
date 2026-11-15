#!/usr/bin/env python3
"""Fix the socket reader stall in a RuneScape client jar from builds 600 to 647.

The reader thread's free-space check counts an empty buffer as full, so the thread can wait forever and
the client reconnects 15 s later. Build 648 fixed the check; this flips the same comparison so an empty
buffer has room."""

import argparse
import struct
from pathlib import Path
from zipfile import ZipFile

# if_icmplt <-> if_icmple and if_icmpge <-> if_icmpgt: only the outcome at equality changes.
FLIP = {0xA1: 0xA4, 0xA4: 0xA1, 0xA2: 0xA3, 0xA3: 0xA2}
TAKEN_AT_EQUAL = {0x9F, 0xA2, 0xA4}
NAMES = {0x9F: "if_icmpeq", 0xA0: "if_icmpne", 0xA1: "if_icmplt", 0xA2: "if_icmpge", 0xA3: "if_icmpgt", 0xA4: "if_icmple"}
GETFIELD, ICONST_M1, INVOKEVIRTUAL = 0xB4, 0x02, 0xB6

LENGTHS = [1] * 256
for op in (0x10, 0x12, *range(0x15, 0x1A), *range(0x36, 0x3B), 0xA9, 0xBC):
    LENGTHS[op] = 2
for op in (0x11, 0x13, 0x14, 0x84, *range(0x99, 0xA9), *range(0xB2, 0xB9), 0xBB, 0xBD, 0xC0, 0xC1, 0xC6, 0xC7):
    LENGTHS[op] = 3
LENGTHS[0xC5] = 4
for op in (0xB9, 0xBA, 0xC8, 0xC9):
    LENGTHS[op] = 5


def u2(data, pos):
    return struct.unpack_from(">H", data, pos)[0]


class ClassFile:
    def __init__(self, data):
        self.data = data
        count = u2(data, 8)
        self.cp = [None] * count
        pos, i = 10, 1
        while i < count:
            tag = data[pos]
            if tag == 1:
                n = u2(data, pos + 1)
                self.cp[i] = data[pos + 3:pos + 3 + n].decode("utf-8", "replace")
                pos += 3 + n
            elif tag in (3, 4):
                pos += 5
            elif tag in (5, 6):
                pos += 9
                i += 1
            elif tag in (7, 8, 16, 19, 20):
                self.cp[i] = (u2(data, pos + 1),)
                pos += 3
            elif tag in (9, 10, 11, 12, 17, 18):
                self.cp[i] = (u2(data, pos + 1), u2(data, pos + 3))
                pos += 5
            elif tag == 15:
                pos += 4
            else:
                raise ValueError(f"Unknown constant pool tag {tag}")
            i += 1
        interfaces = u2(data, pos + 6)
        self.interfaces = [self.cp[self.cp[u2(data, pos + 8 + 2 * k)][0]] for k in range(interfaces)]
        pos += 8 + 2 * interfaces
        pos = self._members(pos)[0]
        _, self.methods = self._members(pos)

    def _members(self, pos):
        members = []
        count = u2(self.data, pos)
        pos += 2
        for _ in range(count):
            name, desc, attributes = self.cp[u2(self.data, pos + 2)], self.cp[u2(self.data, pos + 4)], u2(self.data, pos + 6)
            pos += 8
            code = None
            for _ in range(attributes):
                length = struct.unpack_from(">I", self.data, pos + 2)[0]
                if self.cp[u2(self.data, pos)] == "Code":
                    size = struct.unpack_from(">I", self.data, pos + 10)[0]
                    code = pos + 14, size
                pos += 6 + length
            members.append((name, desc, code))
        return pos, members

    def member(self, index):
        owner, nat = self.cp[index]
        name, desc = self.cp[nat]
        return self.cp[self.cp[owner][0]], self.cp[name], self.cp[desc]

    def int_field(self, ins):
        if ins[1] != GETFIELD:
            return None
        _, name, desc = self.member(u2(ins[2], 0))
        return name if desc == "I" else None


def instructions(code):
    out, pc = [], 0
    while pc < len(code):
        op = code[pc]
        pad = (4 - (pc + 1) % 4) % 4
        if op == 0xAA:
            low, high = struct.unpack_from(">ii", code, pc + 1 + pad + 4)
            size = 1 + pad + 12 + 4 * (high - low + 1)
        elif op == 0xAB:
            size = 1 + pad + 8 + 8 * struct.unpack_from(">i", code, pc + 1 + pad + 4)[0]
        elif op == 0xC4:
            size = 6 if code[pc + 1] == 0x84 else 4
        else:
            size = LENGTHS[op]
        out.append((pc, op, code[pc + 1:pc + size]))
        pc += size
    return out


def is_branch(op):
    return 0x99 <= op <= 0xA6 or op in (0xC6, 0xC7)


def is_store(op):
    return op == 0x36 or 0x3B <= op <= 0x3E


def free_space_check(cls, ins):
    """The read/write comparison in run()'s free-space check: (index, read field), or None."""
    read = zero = None
    prev = 0
    for i, (_, op, _) in enumerate(ins):
        if not is_branch(op):
            continue
        seg = ins[prev:i]
        fields = {f for f in map(cls.int_field, seg) if f}
        negated = sum(1 for x in seg if x[1] == ICONST_M1) >= 2
        if len(fields) == 1 and (op in (0x99, 0x9A) or (op in (0x9F, 0xA0) and negated)):
            read, zero = fields.pop(), i
            break
        prev = i + 1
    if read is None:
        return None
    prev = zero + 1
    for i in range(zero + 1, len(ins)):
        op = ins[i][1]
        if 0x9F <= op <= 0xA4:
            fields = {f for f in map(cls.int_field, ins[prev:i]) if f}
            if read in fields and len(fields) == 2:
                return i, read
        if is_branch(op) or op in (0xA7, 0xC8) or is_store(op):
            prev = i + 1
    return None


def reads_at_equal(cls, ins, at, read):
    """Whether the arm run() takes when read == write is the one that reads `read` (the bug)."""
    def arm(start):
        out = []
        for x in ins[start:]:
            out.append(x)
            if is_store(x[1]):
                break
        return any(cls.int_field(x) == read for x in out)

    pc, op, arg = ins[at]
    target = pc + struct.unpack(">h", arg[:2])[0]
    fall, jump = arm(at + 1), arm(next(k for k, x in enumerate(ins) if x[0] == target))
    if fall == jump:
        raise ValueError("The free-space check's arms don't look like the socket reader's")
    return jump if op in TAKEN_AT_EQUAL else fall


def find_reader(jar):
    """(entry, class bytes, file offset of the comparison, its opcode, buggy) for the jar's reader."""
    found = []
    for name in jar.namelist():
        if not name.endswith(".class"):
            continue
        data = jar.read(name)
        cls = ClassFile(data)
        if "java/lang/Runnable" not in cls.interfaces:
            continue
        for method, desc, code in cls.methods:
            if method != "run" or desc != "()V" or code is None:
                continue
            ins = instructions(data[code[0]:code[0] + code[1]])
            if not any(x[1] == INVOKEVIRTUAL and cls.member(u2(x[2], 0)) == ("java/io/InputStream", "read", "([BII)I") for x in ins):
                continue
            check = free_space_check(cls, ins)
            if check:
                at, read = check
                found.append((name, data, code[0] + ins[at][0], ins[at][1], reads_at_equal(cls, ins, at, read)))
    if not found:
        raise ValueError("No socket reader thread: builds before 600 read the socket on the game thread")
    if len(found) > 1:
        raise ValueError(f"Expected one socket reader thread, found {len(found)}")
    return found[0]


def fix_reader(jar):
    """(entry, fixed class bytes) for a jar whose reader has the bug."""
    name, data, offset, op, buggy = find_reader(jar)
    if not buggy:
        raise ValueError(f"{name}: the socket reader check ({NAMES[op]}) is already fixed")
    fixed = bytearray(data)
    fixed[offset] = FLIP[op]
    fixed = bytes(fixed)
    cls = ClassFile(fixed)
    for method, desc, code in cls.methods:
        if method == "run" and desc == "()V" and code:
            ins = instructions(fixed[code[0]:code[0] + code[1]])
            check = free_space_check(cls, ins)
            if check and reads_at_equal(cls, ins, *check):
                raise ValueError(f"{name}: flipping {NAMES[op]} did not fix the check")
    return name, fixed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("jar", type=Path, help="A client jar from builds 600 to 647")
    parser.add_argument("--output", type=Path, help="Write the fixed jar here; otherwise only report")
    args = parser.parse_args()

    with ZipFile(args.jar) as jar:
        name, _, _, op, buggy = find_reader(jar)
        state = "stalls on an empty buffer" if buggy else "already fixed"
        print(f"{args.jar}: socket reader {name}, check {NAMES[op]}: {state}")
        if not args.output or not buggy:
            return
        reader, fixed = fix_reader(jar)
        with ZipFile(args.output, "w") as out:
            for entry in jar.infolist():
                out.writestr(entry, fixed if entry.filename == reader else jar.read(entry))
    print(f"Wrote {args.output}: only {reader} changed ({NAMES[op]} -> {NAMES[FLIP[op]]})")


if __name__ == "__main__":
    main()
