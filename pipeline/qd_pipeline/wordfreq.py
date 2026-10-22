"""Read wordfreq frequency lists (https://github.com/rspeer/wordfreq).

wordfreq ships its data as gzipped msgpack "cBpack" files
(`wordfreq/data/large_fr.msgpack.gz`). The file holds a list: a header map,
then one list of words per frequency bucket. Bucket i holds the words whose
frequency is 10^(-i/100), so their Zipf value (log10 of occurrences per
billion words) is 9 - i/100.

msgpack is decoded here directly, so the pipeline needs no dependencies.
"""

import gzip
import struct
from pathlib import Path

from .normalize import normalize


class _Reader:
    def __init__(self, data: bytes):
        self.data = data
        self.pos = 0

    def take(self, n: int) -> bytes:
        chunk = self.data[self.pos : self.pos + n]
        self.pos += n
        return chunk

    def uint(self, size: int) -> int:
        return int.from_bytes(self.take(size), "big")

    def read(self):
        b = self.data[self.pos]
        self.pos += 1
        if b <= 0x7F:
            return b
        if 0x80 <= b <= 0x8F:
            return self.map(b & 0x0F)
        if 0x90 <= b <= 0x9F:
            return self.array(b & 0x0F)
        if 0xA0 <= b <= 0xBF:
            return self.take(b & 0x1F).decode("utf-8")
        if b >= 0xE0:
            return b - 0x100
        simple = {0xC0: None, 0xC2: False, 0xC3: True}
        if b in simple:
            return simple[b]
        if b in (0xD9, 0xDA, 0xDB):
            return self.take(self.uint(1 << (b - 0xD9))).decode("utf-8")
        if b in (0xCC, 0xCD, 0xCE, 0xCF):
            return self.uint(1 << (b - 0xCC))
        if b in (0xD0, 0xD1, 0xD2, 0xD3):
            size = 1 << (b - 0xD0)
            return int.from_bytes(self.take(size), "big", signed=True)
        if b == 0xCA:
            return struct.unpack(">f", self.take(4))[0]
        if b == 0xCB:
            return struct.unpack(">d", self.take(8))[0]
        if b in (0xDC, 0xDD):
            return self.array(self.uint(2 if b == 0xDC else 4))
        if b in (0xDE, 0xDF):
            return self.map(self.uint(2 if b == 0xDE else 4))
        raise ValueError(f"unsupported msgpack type 0x{b:02x} at offset {self.pos - 1}")

    def array(self, n: int) -> list:
        return [self.read() for _ in range(n)]

    def map(self, n: int) -> dict:
        return {self.read(): self.read() for _ in range(n)}


def load_zipf(path: Path) -> dict[str, float]:
    """Map each normalized word to its Zipf frequency (0 to about 8)."""
    with gzip.open(path, "rb") as f:
        data = _Reader(f.read()).read()
    header, buckets = data[0], data[1:]
    if header.get("format") != "cB":
        raise ValueError(f"{path} is not a wordfreq cBpack file")
    zipf: dict[str, float] = {}
    for i, words in enumerate(buckets):
        value = round(9 - i / 100, 2)
        for word in words:
            # Buckets go from most to least frequent: keep the first value,
            # so "été" and "ete" share the frequency of the commoner spelling.
            zipf.setdefault(normalize(word), value)
    return zipf
