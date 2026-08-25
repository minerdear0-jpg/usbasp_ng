#ifndef USBASP_TPI_H_
#define USBASP_TPI_H_

#include <stdint.h>

extern uint16_t tpi_dly_cnt;

void tpi_init(void);
void tpi_send_byte(uint8_t b);
uint8_t tpi_recv_byte(void);
void tpi_read_block(uint16_t addr, uint8_t *dptr, uint8_t len);
void tpi_write_block(uint16_t addr, const uint8_t *sptr, uint8_t len);

#endif
