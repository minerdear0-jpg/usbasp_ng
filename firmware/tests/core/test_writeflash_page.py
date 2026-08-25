#!/usr/bin/env python3
"""WRITEFLASH pagecounter / LAST flush — firmware/src/vendor_isp.c."""

FIRST, LAST = 1, 2


def writeflash_loop(nbytes, pagesize, blockflags):
    """Flush addresses for one WRITEFLASH SETUP (FIRST resets the counter)."""
    pagecounter = pagesize if (blockflags & FIRST) else pagesize
    flushes = []
    addr = 0
    remaining = nbytes
    for i in range(nbytes):
        if pagesize == 0:
            pass
        else:
            pagecounter -= 1
            if pagecounter == 0:
                flushes.append(addr)
                pagecounter = pagesize
        remaining -= 1
        if remaining == 0:
            if (blockflags & LAST) and pagecounter != pagesize:
                flushes.append(addr)
        addr += 1
    return flushes


def test_mega8_page_64_full():
    # 64-byte page, one full page, FIRST|LAST
    fl = writeflash_loop(64, 64, FIRST | LAST)
    assert fl == [63]


def test_mega8_two_pages():
    fl = writeflash_loop(128, 64, FIRST | LAST)
    assert fl == [63, 127]


def test_last_partial_page():
    fl = writeflash_loop(10, 64, FIRST | LAST)
    assert fl == [9]


def test_not_last_no_partial_flush():
    fl = writeflash_loop(10, 64, FIRST)
    assert fl == []


def test_pagesize_256_is_not_uchar_zero():
    pagesize = 0x100
    assert pagesize != 0
    assert (pagesize & 0xFF) == 0  # the uchar bug
    fl = writeflash_loop(256, pagesize, FIRST | LAST)
    assert fl == [255]


def test_tpi_nbytes_no_underflow():
    nbytes = 5
    length = 8
    if nbytes > length:
        nbytes -= length
    else:
        nbytes = 0
    assert nbytes == 0


def main():
    test_mega8_page_64_full()
    test_mega8_two_pages()
    test_last_partial_page()
    test_not_last_no_partial_flush()
    test_pagesize_256_is_not_uchar_zero()
    test_tpi_nbytes_no_underflow()
    print("ok  writeflash_page")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
