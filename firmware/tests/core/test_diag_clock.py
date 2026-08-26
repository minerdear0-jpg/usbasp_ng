#!/usr/bin/env python3
"""Host-side model of firmware/src/diag/diag_clock.c (Timer1 + lazy TOV)."""

from __future__ import annotations


class DiagClockModel:
    """Mirrors diag_now() / diag_elapsed() without an overflow ISR."""

    def __init__(self) -> None:
        self.tcnt = 0
        self.tov = False
        self.epoch = 0

    def init(self) -> None:
        self.tcnt = 0
        self.tov = False
        self.epoch = 0

    def advance(self, ticks: int) -> None:
        """Advance free-running TCNT1; set TOV on each wrap."""
        assert ticks >= 0
        for _ in range(ticks):
            self.tcnt = (self.tcnt + 1) & 0xFFFF
            if self.tcnt == 0:
                self.tov = True

    def now(self) -> int:
        cnt = self.tcnt
        if self.tov:
            self.tov = False
            cnt = self.tcnt
            self.epoch = (self.epoch + 1) & 0xFFFF
        return ((self.epoch & 0xFFFF) << 16) | (cnt & 0xFFFF)

    @staticmethod
    def elapsed(start: int, end: int) -> int:
        return (end - start) & 0xFFFFFFFF

    @staticmethod
    def wire16(tick: int) -> int:
        return tick & 0xFFFF


def test_monotonic_simple() -> None:
    c = DiagClockModel()
    c.init()
    a = c.now()
    c.advance(1)
    b = c.now()
    c.advance(0x100)
    d = c.now()
    assert a <= b <= d


def test_wrap_16_extends_epoch() -> None:
    c = DiagClockModel()
    c.init()
    c.advance(0xFFFF)
    before = c.now()
    assert DiagClockModel.wire16(before) == 0xFFFF
    c.advance(1)  # wrap → TOV
    after = c.now()
    assert (after >> 16) == 1
    assert DiagClockModel.wire16(after) == 0
    assert DiagClockModel.elapsed(before, after) == 1


def test_wire16_sequence_docs() -> None:
    """Golden low-16 sequence from the design note."""
    c = DiagClockModel()
    c.init()
    samples = []
    for tcnt, tov_before in (
        (0x0000, False),
        (0x0001, False),
        (0x0002, False),
        (0x0100, False),
        (0xFFFF, False),
        (0x0000, True),  # after wrap, TOV sticky
    ):
        c.tcnt = tcnt
        c.tov = tov_before
        samples.append(f"{DiagClockModel.wire16(c.now()):04X}")
    assert samples == ["0000", "0001", "0002", "0100", "FFFF", "0000"]
    assert c.epoch == 1


def test_elapsed_across_u32_wrap() -> None:
    start = 0xFFFFFFF0
    end = 0x00000010
    assert DiagClockModel.elapsed(start, end) == 0x20


def test_missed_double_overflow_is_documented_limit() -> None:
    """Two wraps without now() only bumps epoch once (lazy TOV limit)."""
    c = DiagClockModel()
    c.init()
    t0 = c.now()
    c.advance(0x10000)  # one wrap
    c.advance(0x10000)  # second wrap; TOV still single sticky bit
    t1 = c.now()
    assert (t1 >> 16) - (t0 >> 16) == 1


def main() -> int:
    test_monotonic_simple()
    test_wrap_16_extends_epoch()
    test_wire16_sequence_docs()
    test_elapsed_across_u32_wrap()
    test_missed_double_overflow_is_documented_limit()
    print("ok  diag_clock")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
