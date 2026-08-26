# Канарейка — smoke loop (ATmega8 on Nano PCB)

Flash via **USBasp2** (ISP) or **Optiboot** (UART / CH340). Observe LEDs + `/dev/ttyUSB0`.

**Канарейка** = this chip. UART banner: `who=canary`. For diag dual-truth see
[`../mega8-diag-oracle/`](../mega8-diag-oracle/) — do not combine with Optiboot
(both want flash `0x1E00`).

Assumes 16 MHz crystal, UART on PD0/PD1 (Nano D0/D1 → CH340). LEDs: D13 + D2/D3/D4 chase (only wired ones light).

## ISP (recovery / first bring-up)

```bash
# terminal A
./dist/diagplane.bin watch --serial YEL0

# terminal B
make -C bench/mega8-nano-loop flash
# then:
screen /dev/ttyUSB0 115200
# RESET on Nano PCB → banner again
```

`make flash` does a chip erase and **removes Optiboot**. Re-burn with `make bootloader` if you want UART uploads again.

## Optiboot (fast UART updates)

One-time (ISP still needed once):

```bash
make -C bench/mega8-nano-loop bootloader
```

Then iterate without re-plugging ISP:

```bash
make -C bench/mega8-nano-loop flash-uart
# UART_PORT=/dev/ttyUSB1 make flash-uart   # if needed
```

Hex provenance: `bootloader/ORIGIN.txt` (MCUdude optiboot_flash, 512 B @ 0x1E00, 115200).
Fuses match MiniCore ATmega8 @ 16 MHz + BOD 2.7 V: `lfuse=0xBF` `hfuse=0xC4`.
Keep YEL0 / classic USBasp available for fuse recovery.
