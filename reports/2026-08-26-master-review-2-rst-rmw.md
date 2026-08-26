# Firmware review 2 — RST RMW vs SW SCK, HID/WCID check

**Accepted:** 2026-08-26  
**Source:** full-tree read against GitHub `master` and Fischl 2011 (`reference/usbasp-2011-05-28/`).  
**Status:** take as guidance. Retracts an earlier SW-SCK sample-phase hypothesis. Proposes a **program-only** RST `cli`/`SREG` experiment before more scope work.

Companion: [`2026-08-26-master-review.md`](2026-08-26-master-review.md). Compatibility contract remains [`docs/COMPATIBILITY.md`](../docs/COMPATIBILITY.md). Parked symptom: [`docs/SOFTWARE_SCK.md`](../docs/SOFTWARE_SCK.md).

---

Прочитан весь код (клон GitHub для сравнения с оригиналом 2011 и с zip). Часть более ранних догадок из переписки про осциллограф была **неверна** — ниже это зафиксировано. Находка: конкретная, проверяемая **без осциллографа** асимметрия в текущем коде, которая ложится на симптом «software SCK всегда FAIL, hardware SPI всегда PASS».

## 1. Отзыв теории «sample-before-shift» в `ispTransmit_sw`

Раньше (до кода) предполагалось, что в софтверном bitbang MISO читается до, а не после фронта SCK, и это баг.

Построчно с эталоном `reference/usbasp-2011-05-28/firmware/isp.c` — **та же последовательность** (sample MISO → потом поднять SCK). NG ничего тут не поменял по существу, только обернул в `cli`/`sei`.

Раз оригинал работает на миллионах клонов ~15 лет — это не баг, это нормальная синхронизация под Mode 0 (target обновляет MISO по заднему фронту, а не по переднему).

**Прежнюю гипотезу отозвать. Гоняться за ней не стоит.** Это противоречит пункту 3 первого отчёта (явный sample-after-rise как «правильная форма»); для NG приоритет у **этого** отзыва, пока нет waveform, который покажет иное.

## 2. Реальная находка: RST toggling не защищён `cli`/`sei`, а MOSI/SCK — защищён

Собственный комментарий в `isp.c` объясняет мотив: V-USB делает TX через `in`/`ori`/`out` (не-атомарный RMW) на тех же `PORTB`/`DDRB`, где сидят ISP-пины, и INT0 может влезть между `in` и `out`, затерев чужой бит. Поэтому в `ispTransmit_sw` каждую запись MOSI/SCK явно оборачивают в `cli()` / `SREG = sreg`.

Та же незащищённая RMW на `PORTB` есть в других местах — **там защиты нет**:

- `ISP_OUT |= (1 << ISP_RST)` / `&= ~(1 << ISP_RST)` — та же 3-инструкционная `in`/`ori`(`andi`)/`out` последовательность на PORTB, которую комментарий в `ispTransmit_sw` называет опасной.

Почему это правдоподобно объясняет именно **HW PASS / SW FAIL**, а не случайные глюки:

- RST дёргается **один раз** на попытку и при HW, и при SW — сам факт незащищённости одинаков.
- **Окно уязвимости у SW-пути на порядки длиннее.** HW SPI: байт за единицы микросекунд (1.5 MHz). SW `-B 22`: 8 бит × (16 µs + 16 µs) ≈ 256 µs на байт, транзакция `AC 53 00 00` — больше 1 мс, плюс 20 мс ожидания после RST. USB-хост шлёт SOF/IN/OUT ~каждую 1 мс, INT0 стабильно стреляет, V-USB в ISR делает RMW на PORTB. У HW окно исчезающе мало, у SW растянуто.
- Один прилёт ISR между `in` и `out` записи RST: RST либо не опустится, либо не поднимется как задумано → ENABLEPROG `0x01`, USB не отваливается (стек цел, ISP-хендшейк не прошёл).

Проверка **чисто программно**, без осциллографа и третьей платы: обернуть RST-тоглы в тот же `cli()` / `SREG` паттерн, что для MOSI/SCK, и прогнать `-B 22` заново.

Шаблон для `ispConnect()` (как в отчёте): DDR/MOSI/SCK/MISO/clockWait как сейчас; **только** два RST-тогла (release then assert) внутри `cli`/`SREG`. То же — вокруг двух RST-тоглов внутри `ispEnterProgrammingMode()` (`do { ... } while (--tries)`) и вокруг RST в `USBASP_FUNC_TPI_CONNECT` / `TPI_DISCONNECT` (`vendor_isp.c`).

Критерий:

- Если `-B 22` / `-B 50` / `-B 250` начнут проходить (или перестанут падать 100% стабильно) — причина найдена без измерения.
- Если не изменится — дыра всё равно закрыта (RST должен быть защищён по той же логике, что MOSI/SCK); тогда сниффер/осциллограф оправдан уже с исключённым этим источником.

## 3. Что уже сделано правильно (Windows / современные ОС)

### HID `usb_setup.c`

GET_IDLE / SET_IDLE / GET_PROTOCOL / SET_PROTOCOL. По HID 1.11: SET_IDLE — wValue hi (`data[3]`) = Duration, lo (`data[2]`) = Report ID; SET_PROTOCOL — wValue lo (`data[2]`) = 0/1. Верно.

Заодно исправлен структурный баг: раньше весь `switch(data[1])` был внутри `if (data[3]==3)` (Feature), то есть GET/SET_REPORT для Input/Output молча игнорировались. Вынос `switch` наружу — правильный фикс диспетчеризации, не только новые кейсы.

Мотивация: `usbhid.sys` на Win10/11 при биндинге HID штатно шлёт SET_IDLE / иногда GET_PROTOCOL; STALL/пустой ответ — классическая причина зависаний енумерации композитных V-USB. Грабля обойдена на будущее.

### MS OS 2.0 (`usb_descriptors.h`)

Пересчёт байт-в-байт: wTotalLength сходится на Set Header → Configuration Subset → Function Subset. `DeviceInterfaceGUIDs` (REG_MULTI_SZ) только на vendor IF0; HID IF1/IF2 больше не получают WinUSB-связывание. Тест `test_msos20_winusb_only_if0` закрепляет это.

`bcdDevice`: classic `0x0200`, HIDUART `0x0201`, общий VID/PID `16c0:05dc` — чтобы Driver Store не путал два образа.

## 4. Что ещё подстраховать

**Watchdog при старте `main()` отсутствует.** Ни `MCUSR = 0`, ни `wdt_disable()`. На classic ATmega8 низкий риск (нет erratum «watchdog переживает soft-reset»). Сборка `usbasp-atmega88`: у 88/168/328 WDT умеет пережить сброс и зациклить bootloop, если когда-либо включён. Дешёвая страховка в начало `main()`:

```c
MCUSR = 0;
wdt_disable();
```

(`#include <avr/wdt.h>`). Не критично сейчас; типичная грабля при переносе на 88/168/328.

**`board_led_usb_update()`** дёргает `ispSetSCKOption()` из главного цикла при смене JP3. Логика ок (однопоточно с `usbFunctionSetup`), но это ещё одна точка, где `SPCR` / `sck_sw_delay` / `ispTransmit` меняются вне ISP-transmit пути. При другом приоритете IRQ — перепроверить.

## Коротко

Главный «грабель» из `SOFTWARE_SCK.md` («parked, not scoped») можно попробовать закрыть **без измерения**: RST-тоглы в `cli`/`sei` по аналогии с MOSI/SCK. HID Idle/Protocol и MS OS 2.0 перепроверены по спекам и арифметике; дополнительных дыр там рецензент не нашёл.
