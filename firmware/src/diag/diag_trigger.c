#include "usbasp_config.h"

#if USBASP_HAS_DIAG

#include "diag/diag_trigger.h"
#include "diag/diag_events.h"

static diag_trigger_t trig_cfg;

void diag_trigger_init(void)
{
    diag_trigger_set_enableprog_fail();
}

void diag_trigger_set(const diag_trigger_t *cfg)
{
    if (!cfg) {
        trig_cfg.kind = DIAG_TRIG_NONE;
        return;
    }
    trig_cfg = *cfg;
}

void diag_trigger_get(diag_trigger_t *out)
{
    if (out)
        *out = trig_cfg;
}

void diag_trigger_set_enableprog_fail(void)
{
    trig_cfg.kind = DIAG_TRIG_ENABLEPROG_FAIL;
    trig_cfg.type = DIAG_ENABLEPROG;
    trig_cfg.flags = 0;
    trig_cfg.a = 0;
    trig_cfg.b = 0;
}

bool diag_trigger_match(const diag_frame_t *frame, const diag_trigger_t *cfg)
{
    if (!frame || !cfg)
        return false;

    switch (cfg->kind) {
    case DIAG_TRIG_NONE:
        return false;
    case DIAG_TRIG_EVENT_TYPE:
        return frame->type == cfg->type;
    case DIAG_TRIG_EVENT_TYPE_FLAGS:
        return frame->type == cfg->type
            && (frame->flags & cfg->flags) == cfg->flags;
    case DIAG_TRIG_ENABLEPROG_FAIL:
        return frame->type == DIAG_ENABLEPROG
            && (frame->flags & DIAG_EP_END) != 0
            && (frame->flags & DIAG_EP_RESULT_FAIL) != 0;
    case DIAG_TRIG_TRACE_OVERFLOW:
        return frame->type == DIAG_TRACE_OVERFLOW;
    default:
        return false;
    }
}

const diag_trigger_t *diag_trigger_cfg(void)
{
    return &trig_cfg;
}

#endif
