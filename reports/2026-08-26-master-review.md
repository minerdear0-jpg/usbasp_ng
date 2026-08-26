# Firmware review — master after HIDUART WCID / hidraw

**Accepted:** 2026-08-26  
**Scope:** current `master` (not the prior commit-smoke review).  
**Status:** take as guidance. Do not treat this file as a compatibility contract; that remains [`docs/COMPATIBILITY.md`](../docs/COMPATIBILITY.md).

Scores from the reviewer: architecture 8/10, compatibility discipline 9/10, build 8/10, ISP state model 7/10, SW SCK 6/10, HID separation 9/10.

---

Посмотрел текущий master, уже не старую версию. И вот теперь можно делать настоящий review, а не гадать по дыму от предыдущего коммита. Репозиторий заметно повзрослел: `firmware/`, `reference/`, `docs/`, `tests/`, отдельный `src_hid/`, board profiles, compatibility contract, CI. Это уже действительно похоже на проект, а не на очередной USBasp-форк с философией «оно же собирается».

Но есть несколько важных замечаний, и одно из них прямо касается истории с software SCK.

## 1. Главный баг сейчас: комментарий про I=1 правильный, но архитектурная модель ещё не доведена

В `usb_setup.c` прямо зафиксировано:

`usbFunctionSetup()` runs in `usbPoll()` with I=1

Это правильная модель для этой архитектуры: `usbPoll()` вызывается из main context, а INT0 только обслуживает USB событие. Поэтому `usbFunctionSetup()` действительно может быть прерван следующим INT0.

И `sck.c` теперь это учитывает: `cli()`; PORTB RMW; `SREG = sreg`; а задержка выполняется уже с разрешёнными IRQ.

Это хорошее решение.

Не оставлять комментарий `/* setup runs from usbPoll, I=1 */` как просто пояснение к `cli()`. Это уже архитектурный инвариант. Его надо поднять в документацию:

```
USB execution model:

INT0 ISR
    ↓
sets/handles USB event
    ↓
returns
    ↓
main → usbPoll()
    ↓
usbFunctionSetup()
    ↓
USBasp operation
```

И отдельно:

- ISP operation may be preempted by INT0.
- PORTB RMW critical sections must therefore be atomic.
- Timing delays must tolerate ISR extension.

Это гораздо сильнее.

## 2. Почему предыдущая гипотеза была только наполовину правильной

Сейчас: SCK_HIGH → `SREG = sreg` → `delay()`. INT0 действительно может войти внутрь high phase. Но он может только сделать `16 µs + USB ISR + 16 µs` — то есть **растянуть** high, а не укоротить.

Это важно для SCK: target видит растянутый high, а не unexpectedly shorter high.

Поэтому текущий цикл уже имеет хорошее свойство: ISR jitter может уменьшить effective SCK frequency, но не нарушить minimum high/low timing.

Это прямо написать в [`docs/SOFTWARE_SCK.md`](../docs/SOFTWARE_SCK.md) как **formal timing guarantee**.

## 3. В `ispTransmit_sw()` всё ещё неприятная фаза MISO

Сейчас: set MOSI → sample MISO → set SCK high → restore IRQ → delay → set SCK low → delay.

MISO читается **BEFORE rising SCK**.

Для AVR SPI mode 0 это не обязательно функциональный баг: slave уже держит текущий MISO bit стабильным в low phase. Но это плохая форма для timing contract.

Сделать явно:

- MOSI setup → SCK rising → sample MISO → SCK falling

или, если сохранить текущую фазовую модель:

- MOSI → delay/setup → SCK ↑ → sample MISO → high delay → SCK ↓ → low delay

И тогда waveform становится очевидным. Сейчас код заставляет будущего ревьюера делать mental gymnastics с SPI mode 0.

## 4. PORTB RMW атомарен, но timing не deterministic

`cli(); ISP_OUT |= ...; SREG = sreg;` защищает read-modify-write PORTB от INT0. Это правильно.

Но SCK HIGH → interrupts enabled → delay, поэтому внутри high phase возможна произвольная V-USB latency.

Это допустимо только если явно определить software SCK как:

**`f_requested` = upper-bound target frequency, not exact frequency.**

SW SCK contract:

- minimum half-period = requested half-period
- actual half-period >= minimum
- ISR may only stretch

## 5. Тестировать не математикой, а осциллографом

В README есть 32 / 16 / 4 kHz и фиксированная проблема `0x01`. Добавить в `SOFTWARE_SCK.md` таблицу:

| Requested | min half | measured high | measured low | max jitter |
|-----------|----------|---------------|--------------|------------|
| 32 kHz | 16 µs | ? | ? | ? |
| 16 kHz | 32 µs | ? | ? | ? |
| 4 kHz | 128 µs | ? | ? | ? |
| 8 kHz JP3 | 62.5 µs | ? | ? | ? |

Пока этих данных нет, утверждение «cycle-count half-period (INT0 may stretch)» архитектурно правильное, но ещё не экспериментально доказанное.

## 6. LED вынесен из SW bitbang — оставить

В `ispTransmit_hw()` LED всё ещё вызывается (`board_led_isp_activity()`), в software path его нет. `COMPATIBILITY.md` это фиксирует: LED stays out of `ispTransmit_sw`. Оставить.

Следующий шаг: `board_led_isp_activity()` внутри hardware SPI тоже не обязан находиться на каждом байте. Activity notification должна быть на уровне операции, а не транспортного байта:

```
ISP operation
 ├── SPI transaction
 └── activity notification
```

## 7. `isp.c` всё ещё мини-монолит

`isp.c` знает: target protocol, SCK selection, hardware SPI, software SPI, board jumper, LED, RESET, auto slowdown, AT89 special case.

Следующим этапом (без enterprise cathedral на ATmega8) достаточно:

- `isp_transport_tx()`
- `isp_transport_set_clock()`
- `isp_transport_connect()`

и разнести session / transport / `sck.c` / `board.c`.

## 8. Выделить ISP transport

Фактически уже есть два транспорта (HW SPI / SW SPI), выраженные через `uchar (*ispTransmit)(uchar)`. Слишком слабая abstraction boundary.

Предложение:

```c
typedef uchar (*isp_transfer_fn)(uchar);

struct isp_transport {
    isp_transfer_fn transfer;
    void (*enable)(void);
    void (*disable)(void);
};
```

Тогда `ispEnterProgrammingMode()` перестаёт знать детали SPCR, SPSR и software bitbang.

## 9. Скрытый баг state machine: `prog_sck` перегружен

В `ispEnterProgrammingMode()` `prog_sck` является одновременно requested host setting и current effective setting. Это тот класс вещей, который рождает старый bug: DISCONNECT неожиданно меняет выбранный SCK.

Разделить:

- `uint8_t requested_sck` — what avrdude asked for
- `uint8_t effective_sck` — what current session is actually using

AUTO: requested = AUTO, effective = 1500; после failure effective = 375; после reconnect effective = 1500.

## 10. Это особенно важно для JP3

Сейчас jumper force 8 kHz на проводе и restore `prog_sck` на disconnect работает по текущей модели, но semantic distinction надо документировать:

```
requested_sck
    ↓
session policy
    ├── jumper override
    └── AUTO slowdown
          ↓
effective_sck
```

## 11. Compatibility contract не трогать; добавить L2.5

L0–L3 и правило «fix a bug only if it is not observable compatibility behaviour» не трогать.

Добавить **L2.5: timing contract**:

- HW mode: hardware SPI semantics
- SW mode: minimum half-period guarantee
- interrupt latency may stretch periods
- no ISR may shorten SCK phase

Software SCK уже достаточно важен для собственной категории.

## 12. TPI advertised, silicon не проверен

`USBASP_CAP_TPI` компилируется и рекламируется, но TPI не exercised on silicon. Для development нормально. Для release firmware не делать TPI capability условием compile-time board capability, пока не пройден хотя бы один реальный ATtiny10.

Иначе host видит «I support TPI», а firmware говорит «ну теоретически да». Лучше `USBASP_HAS_TPI` и board profile включает его только после hardware validation.

## 13. Двойная система сборки

Одновременно `scripts/build.sh`, `firmware/Makefile`, cmake. Постепенно: CMake = canonical, `scripts/build.sh` = friendly wrapper; Makefile — compatibility shim или удалить. Иначе через полгода три пути начнут отличаться.

## 14. Fuses: явный confirm оставить

`make fuses` заблокирован до `CONFIRM_FUSES=1`. Оставить обязательно.

Fuse profile — часть board profile (например `boards/.../fuses.txt`), никогда не вычислять fuse из MCU автоматически. Клоны уже показали: 2011 `hfuse=c9`, bench `hfuse=d9`. MCU == ATmega8 недостаточно для fuse policy.

## 15. Reference tree правильный

`firmware/` only build input, `reference/` immutable snapshots, `docs/` compatibility contract. Завершённое решение.

CI: запретить случайно компилировать что-либо из `reference/` (тест, что `reference/` не входит в compiler inputs).

## 16. Следующий commit — не HID

Не HID. Не новые функции. Не новый protocol.

**Commit: `sck: formalize software timing contract`**

Состав:

1. `requested_sck` / `effective_sck`
2. SW SCK timing contract
3. measured waveform table
4. remove any unnecessary calls from timing path
5. atomic PORTB primitives
6. comment execution model
7. add SW-SCK regression notes

И не менять пока сам algorithm, пока осциллограф не даст SCK / MOSI / MISO / RST на `-B 22`.

## Главное замечание по текущей проблеме SW SCK

После просмотра нового кода акцент меняется.

Уже есть: ISR-safe PORTB RMW; cycle-count delay; отсутствие LED в SW path; сохранение requested SCK; отдельный classic/HID; documented fact, что одинаковый SW-SCK failure на classic и HIDUART.

HIDUART больше не подозреваемый №1.

Следующий эксперимент — не «ещё один способ отключить IRQ», а логический анализ четырёх линий на одном `0xAC 0x53 0x00 0x00`. Особенно:

- MOSI `AC`
- MOSI `53`
- MISO `??`
- MISO `53`

на первом rising edge каждого бита.

Если там всё идеально, следующий кандидат уже не USB и не IRQ, а конкретная электрическая/фазовая разница между HW SPI и SW SPI.

## Итог

Архитектура уже реально хорошая. `COMPATIBILITY.md` сделан так, как должен быть сделан такой проект. Убрать двойственность CMake/Make. Разделить `requested_sck` / `effective_sck`. SW SCK: правильная концепция, но ещё нет формального timing contract и hardware proof. HID separation сделана правильно.

Главное: проект вышел из стадии «чинить USBasp». Появляется собственная архитектура; с этого момента строже к state ownership, timing contracts и invariants, чем к количеству строк. Это момент, где firmware либо становится действительно хорошим embedded, либо через год — ещё один очень умный комок `#ifdef`.
