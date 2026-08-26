#!/usr/bin/env python3
"""AUTO backoff and software-SCK delay table vs firmware/src/sck.c."""

AUTO = 0
SCK_0_5 = 1
SCK_1 = 2
SCK_2 = 3
SCK_4 = 4
SCK_8 = 5
SCK_16 = 6
SCK_32 = 7
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


def sw_delay(option):
    return 3 << (SCK_32 - option)


def test_autoslow_seq():
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


def test_sw_delay_matches_fischl_table():
    assert sw_delay(SCK_32) == 3
    assert sw_delay(SCK_16) == 6
    assert sw_delay(SCK_8) == 12
    assert sw_delay(SCK_8) == (3 << (SCK_32 - SCK_8))
    assert sw_delay(SCK_4) == 24
    assert sw_delay(SCK_2) == 48
    assert sw_delay(SCK_1) == 96
    assert sw_delay(SCK_0_5) == 192


def test_hw_threshold():
    assert all(i >= SCK_93_75 for i in range(8, 14))
    assert all(i < SCK_93_75 for i in range(1, 8))
    assert SCK_8 < SCK_93_75  # JP3 8 kHz is software SPI


def test_connect_does_not_store_jumper_as_host_sck():
    from pathlib import Path

    text = Path(__file__).resolve().parents[2] / "src" / "vendor_isp.c"
    src = text.read_text()
    assert "prog_sck = USBASP_ISP_SCK_8" not in src
    assert "isp_apply_host_sck()" in src


def test_rst_portb_uses_cli():
    from pathlib import Path

    root = Path(__file__).resolve().parents[2] / "src"
    isp = (root / "isp.c").read_text()
    vendor = (root / "vendor_isp.c").read_text()
    assert "void isp_out_set_bit" in isp
    assert "void isp_out_clr_bit" in isp
    assert "ISP_OUT |= (1 << ISP_RST)" not in isp
    assert "ISP_OUT &= ~(1 << ISP_RST)" not in isp
    assert "ISP_OUT |= (1 << ISP_RST)" not in vendor
    assert "ISP_OUT &= ~(1 << ISP_RST)" not in vendor
    for name in (
        "isp_out_set_bit(ISP_RST)",
        "isp_out_clr_bit(ISP_RST)",
    ):
        assert name in isp
        assert name in vendor


def test_enableprog_does_not_store_effective_as_requested():
    from pathlib import Path

    root = Path(__file__).resolve().parents[2]
    isp = (root / "src" / "isp.c").read_text()
    sck = (root / "src" / "sck.c").read_text()
    assert "prog_sck = sck" not in isp
    assert "isp_apply_host_sck" in sck
    assert "effective_sck = option" in sck
    assert "board_led_isp_activity" not in isp.split("ispTransmit_hw", 1)[1].split("ispEnterProgrammingMode", 1)[0]
    assert "isp_bus.enable()" in isp
    assert "if (ispTransmit == ispTransmit_hw)" not in isp


def main():
    test_autoslow_seq()
    test_sw_delay_matches_fischl_table()
    test_hw_threshold()
    test_connect_does_not_store_jumper_as_host_sck()
    test_rst_portb_uses_cli()
    test_enableprog_does_not_store_effective_as_requested()
    print("ok  sck_autoslow")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
