#ifndef USBASP_DIAG_TRIGGER_H_
#define USBASP_DIAG_TRIGGER_H_

#include "usbasp_config.h"
#include "diag/diag_ring.h"

#include <stdint.h>
#include <stdbool.h>

/*
 * Trigger layer: fire / don't fire only.
 * Knows nothing about HID, ISP execution, or ring storage layout.
 * Evaluated after diag_trace_push() so the matching event is in capture.
 */

#define DIAG_TRIG_NONE              0
#define DIAG_TRIG_EVENT_TYPE        1
#define DIAG_TRIG_EVENT_TYPE_FLAGS  2
#define DIAG_TRIG_ENABLEPROG_FAIL   3
#define DIAG_TRIG_TRACE_OVERFLOW    4

#ifndef USBASP_DIAG_POST_CAPTURE_EVENTS
#define USBASP_DIAG_POST_CAPTURE_EVENTS 16
#endif

typedef struct {
    uint8_t kind;  /* DIAG_TRIG_* */
    uint8_t type;  /* for EVENT_TYPE / EVENT_TYPE_FLAGS */
    uint8_t flags; /* for EVENT_TYPE_FLAGS: required bits */
    uint8_t a;     /* reserved / match a (0 = don't care) */
    uint8_t b;     /* reserved / match b (0 = don't care) */
} diag_trigger_t;

#if USBASP_HAS_DIAG

void diag_trigger_init(void);
void diag_trigger_set(const diag_trigger_t *cfg);
void diag_trigger_get(diag_trigger_t *out);

/* Default lab predicate: ENABLEPROG_FAIL. */
void diag_trigger_set_enableprog_fail(void);

/* Pure match — no I/O. */
bool diag_trigger_match(const diag_frame_t *frame, const diag_trigger_t *cfg);

/* Active config (for ring lifecycle). */
const diag_trigger_t *diag_trigger_cfg(void);

/*
 * Called after a successful non-meta ring push while ARMED/POST.
 * May advance ARMED → POST → FROZEN.
 */
void diag_trigger_on_event(const diag_frame_t *frame, uint16_t write_index);

#endif

#endif
