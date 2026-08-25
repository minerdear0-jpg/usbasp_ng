#ifndef USBASP_ENDIAN_H_
#define USBASP_ENDIAN_H_

#include <stdint.h>

static inline uint16_t usbasp_read_le16(const uint8_t *p)
{
    return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}

static inline uint32_t usbasp_read_le32(const uint8_t *p)
{
    return (uint32_t)p[0]
         | ((uint32_t)p[1] << 8)
         | ((uint32_t)p[2] << 16)
         | ((uint32_t)p[3] << 24);
}

#endif /* USBASP_ENDIAN_H_ */
