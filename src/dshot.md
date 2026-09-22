# Dshot

For Dshot600 bit period ≈ 1.667 µs

```text
1: ┌──────────────┐
   │              │
   │              └────
   <---- ~75% ---->

0: ┌───────┐
   │       │
   │       └────────────
   <- ~37.5% ->

600 kbit/s × 8 = 4.8 MHz
1 tick = 208.333 ns
0: 3 ticks = 625 ns
1: 6 ticks = 1.250 µs
```

| DShot | required | 8-tick representation    |
|-------|----------|--------------------------|
| bit   | 1.667 us | 8 × 208.33 ns = 1.667 us |
| T0H   | 0.625 us | 3 × 208.33 ns = 0.625 us |
| T1H   | 1.250 us | 6 × 208.33 ns = 1.250 us |

16 bits × 8 ticks = 128 DMA transfers
DshotBuffer [u32; 128]

For 0

|tick  |  0  |  1  |  2  |  3    |  4  |  5  |  6  |  7  |
|------|-----|-----|-----|-------|-----|-----|-----|-----|
| BSRR | SET | NOP | NOP | RESET | NOP | NOP | NOP | NOP |

For 1

|tick  |  0  |  1  |  2  |  3  |  4  |  5  |   6   |  7  |
|------|-----|-----|-----|-----|-----|-----|-------|-----|
| BSRR | SET | NOP | NOP | NOP | NOP | NOP | RESET | NOP |

```text
SET = pin_mask
RESET = pin_mask << 16
NOP = 0
```

That's exactly what GPIO BSRR is designed for:

For example, if motors 1 and 3 are sending 1:

```text
bits  0..15  = set corresponding GPIO pins
bits 16..31  = reset corresponding GPIO pins

SET:

PB6 + PB0

0x0000_0041

and later:

RESET:

0x0041_0000
```
