# Closed-loop bench target (ATmega8 on Nano PCB)

Flash via **USBasp2**, observe LEDs + `/dev/ttyUSB0` (CH340).

```bash
# terminal A
./dist/diagplane.bin watch --serial YEL0

# terminal B
make -C bench/mega8-nano-loop flash
# then:
screen /dev/ttyUSB0 115200
# RESET button on Nano PCB → banner again
```

Assumes 16 MHz crystal, UART on PD0/PD1 (Nano D0/D1 → CH340). LEDs: D13 + D2/D3/D4 chase (only wired ones light).
