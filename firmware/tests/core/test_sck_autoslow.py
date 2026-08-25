#!/usr/bin/env python3
"""AUTO backoff steps must match firmware/src/sck.c isp_sck_autoslow()."""

AUTO = 0
SCK_0_5 = 1
SCK_16 = 6
SCK_93_75 = 8
SCK_375 = 10
SCK_1500 = 12


def autoslow(sck):
    if sck > SCK_375:
        return SCK_375
    if sck > SCK_93_75:
        return SCK_93_75
    if sck > SCK_16:
        return SCK_16
    if sck > SCK_0_5:
        return SCK_0_5
    return AUTO


def main():
    seq = []
    sck = SCK_1500
    while sck >= SCK_0_5:
        seq.append(sck)
        nxt = autoslow(sck)
        if nxt < SCK_0_5:
            break
        sck = nxt
    assert seq == [SCK_1500, SCK_375, SCK_93_75, SCK_16, SCK_0_5], seq
    assert autoslow(SCK_0_5) == AUTO
    print("ok  sck_autoslow")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
