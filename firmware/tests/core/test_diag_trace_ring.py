#!/usr/bin/env python3
"""Model of unified lossy TRACE ring (overwrite + deferred OVERFLOW marker)."""
from __future__ import annotations


class TraceRing:
    def __init__(self, slots: int) -> None:
        assert slots > 0 and (slots & (slots - 1)) == 0
        self.slots = slots
        self.buf: list[int | str] = [0] * slots
        self.head = 0
        self.tail = 0
        self.write_index = 0
        self.dropped = 0
        self.overflow_sticky = False
        self.overflow_marker = False

    def __len__(self) -> int:
        return (self.head - self.tail) & 0xFF  # mimic uint8 distance for small tests

    def _len(self) -> int:
        return self.head - self.tail

    def push(self, item: int | str) -> None:
        if self.overflow_marker and self._len() < self.slots:
            self._write("OVERFLOW")
            self.dropped = 0
            self.overflow_marker = False
        if self._len() >= self.slots:
            self.tail += 1
            self.dropped = min(255, self.dropped + 1)
            self.overflow_sticky = True
            self.overflow_marker = True
        self._write(item)

    def _write(self, item: int | str) -> None:
        self.buf[self.head & (self.slots - 1)] = item
        self.head += 1
        self.write_index += 1

    def drain_all(self) -> list[int | str]:
        out: list[int | str] = []
        while self._len() > 0:
            out.append(self.buf[self.tail & (self.slots - 1)])
            self.tail += 1
        if self.overflow_marker:
            out.append("OVERFLOW")
            self.dropped = 0
            self.overflow_marker = False
        return out


def test_hostile_wrap_slots8() -> None:
    r = TraceRing(8)
    for i in range(11):  # 0..10
        r.push(i)
    # After 0..7 fill, 8 overwrites 0, 9 overwrites 1, 10 overwrites 2
    # Occupancy still 8: values 3..10
    assert r._len() == 8
    assert r.overflow_sticky is True
    # Next push with space only after drain — first insert OVERFLOW then 11
    # Drain without extra push: marker emitted at end
    got = r.drain_all()
    assert got[:8] == [3, 4, 5, 6, 7, 8, 9, 10]
    assert got[-1] == "OVERFLOW"
    assert r.overflow_sticky is True  # sticky until TRACE_END (model keeps it)


def test_marker_between_events() -> None:
    r = TraceRing(4)
    for i in range(6):
        r.push(i)
    # full of 2,3,4,5; marker pending
    r.tail += 1  # simulate one drain → space
    r.push(6)
    # should have written OVERFLOW then 6
    # remaining unread from before: 3,4,5 then OVERFLOW, 6
    got = []
    while r._len() > 0:
        got.append(r.buf[r.tail & (r.slots - 1)])
        r.tail += 1
    assert "OVERFLOW" in got
    assert got[-1] == 6
    assert got[got.index("OVERFLOW") + 1] == 6


def test_no_block() -> None:
    r = TraceRing(4)
    for i in range(100):
        r.push(i)
    assert r._len() == 4
    assert r.overflow_sticky


def main() -> int:
    test_hostile_wrap_slots8()
    test_marker_between_events()
    test_no_block()
    print("ok  diag_trace_ring")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
