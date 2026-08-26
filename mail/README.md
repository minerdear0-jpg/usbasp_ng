# Team mail (plain text)

No SMTP. One file = one letter. **Read only your drop folder.**

| Address | Role | You read |
|---------|------|----------|
| `dev@mail.local` | diagplane / USBasp2 firmware+host | `mail/to-dev/` |
| `deep@mail.local` | Канарейка / target | `mail/to-deep/` |
| `ui@mail.local` | designer — TUI / presentation | `mail/to-ui/` |
| `target@mail.local` | alias → deep | same as deep |

**Канарейка** — ATmega8 on Nano PCB. **Клетка** — YEL0 + ISP ribbon + CH340.
Flash pages `@ 0x1E00` in oracle ≠ the bird’s name.

## Drop (do this)

```text
write  →  mail/to-<recipient>/YYYY-MM-DD_HHMM_<from>_<to>_<slug>.txt
read   →  only mail/to-<you>/   (newest filename first)
done   →  move to mail/archive/
```

`<from>`/`<to>` = local-part only (`deep`, `dev`, `ui`). Slug: kebab-case, ≤40 chars.

**Do not** copy the same letter into inbox/ and outbox/. Those dirs are leftover.

## Headers (exact order)

```text
From: deep@mail.local
To: dev@mail.local
Date: 2026-08-26 23:32 +0600
Type: FYI
Subject: [FYI] short slug here
Re: 2026-08-26_2330_dev_deep_ack-two-costumes.txt
```

`Re:` = parent filename, or `-` if new thread. Then blank line, then body.

| Type | When | Subject prefix |
|------|------|----------------|
| `ASK` | you need an answer | `[ASK]` |
| `DECIDE` | pick A/B, no essay | `[DECIDE]` |
| `ACK` | received, no action | `[ACK]` |
| `FYI` | facts, no reply owed | `[FYI]` |
| `BLOCK` | I am stuck on you | `[BLOCK]` |

## Body (scan in 10 s)

Skip empty sections. Wrap ≤72. **≤25 lines** after headers. More → path to a file.

```text
TL;DR
  one sentence: what changed / what you want

NEED
  - checkbox for the recipient only (omit if ACK/FYI)

FACT
  - new measurements, paths, numbers — no recap they already have

DONT
  - hard constraints (omit if none)

NEXT
  - I will … / waiting on …
```

Copy-paste: [`TEMPLATE.txt`](TEMPLATE.txt).

## Speed rules

1. **TL;DR is the letter.** If they only read that line, they still know the point.
2. No greeting, no “Hi —”, no recap of the previous letter.
3. Numbers over adjectives. Paths over “the firmware”.
4. One topic per file. New topic → new file, not a PS.
5. Russian or English, not both in one letter.
6. `Type:` + subject prefix must match.
