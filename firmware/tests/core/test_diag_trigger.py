#!/usr/bin/env python3
"""Model: ENABLEPROG_FAIL trigger after push → POST → FROZEN."""
from __future__ import annotations

EP_END = 0x04
EP_FAIL = 0x20
EP_OK = 0x10


class Frame:
    def __init__(self, typ: int, flags: int = 0, t: int = 0) -> None:
        self.type = typ
        self.flags = flags
        self.t = t


class Capture:
    def __init__(self, post_n: int = 16) -> None:
        self.post_n = post_n
        self.state = "ARMED"
        self.events: list[Frame] = []
        self.triggered = False
        self.post_left = 0
        self.post_collected = 0
        self.trigger_index = 0

    def push(self, f: Frame) -> bool:
        if self.state == "FROZEN":
            return False
        self.events.append(f)
        idx = len(self.events)
        if self.state == "ARMED":
            if f.type == 6 and (f.flags & EP_END) and (f.flags & EP_FAIL):
                self.triggered = True
                self.trigger_index = idx
                self.state = "POST"
                self.post_left = self.post_n
                self.post_collected = 0
                if self.post_n == 0:
                    self.state = "FROZEN"
            return True
        if self.state == "POST":
            if self.post_left > 0:
                self.post_left -= 1
                self.post_collected += 1
            if self.post_left == 0:
                self.state = "FROZEN"
            return True
        return True


def test_pass_no_fire() -> None:
    c = Capture(16)
    for fl in (0x01, 0x02, 0x02, EP_END | EP_OK):
        assert c.push(Frame(6, fl))
    assert c.state == "ARMED"
    assert not c.triggered


def test_fail_fires_and_post() -> None:
    c = Capture(4)
    for fl in (0x01, 0x02, 0x02, EP_END | EP_FAIL):
        assert c.push(Frame(6, fl))
    assert c.triggered
    assert c.state == "POST"
    assert c.trigger_index == 4  # FAIL frame included
    for i in range(4):
        assert c.push(Frame(9, 0x01 if i == 0 else 0x02))
    assert c.state == "FROZEN"
    assert c.post_collected == 4
    assert c.push(Frame(4, 0x02)) is False  # frozen rejects


def test_fail_event_present() -> None:
    c = Capture(2)
    c.push(Frame(1))
    c.push(Frame(6, EP_END | EP_FAIL, t=0x81A2))
    assert any(e.type == 6 and (e.flags & EP_FAIL) for e in c.events)


def main() -> int:
    test_pass_no_fire()
    test_fail_fires_and_post()
    test_fail_event_present()
    print("ok  diag_trigger")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
