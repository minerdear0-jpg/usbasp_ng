#ifndef USBASP_TPI_DEFS_H_
#define USBASP_TPI_DEFS_H_

#define TPI_OP_SLD      0x20
#define TPI_OP_SLD_INC  0x24
#define TPI_OP_SST      0x60
#define TPI_OP_SST_INC  0x64
#define TPI_OP_SSTPR(a) (0x68 | (a))
#define TPI_OP_SIN(a)   (0x10 | (((a)<<1)&0x60) | ((a)&0x0F))
#define TPI_OP_SOUT(a)  (0x90 | (((a)<<1)&0x60) | ((a)&0x0F))
#define TPI_OP_SLDCS(a) (0x80 | ((a)&0x0F))
#define TPI_OP_SSTCS(a) (0xC0 | ((a)&0x0F))
#define TPI_OP_SKEY     0xE0

#define TPIIR  0xF
#define TPIPCR 0x2
#define TPISR  0x0

#define TPISR_NVMEN 0x02

#define NVMCSR 0x32
#define NVMCMD 0x33
#define NVMCSR_BSY 0x80
#define NVMCMD_NOP           0x00
#define NVMCMD_CHIP_ERASE    0x10
#define NVMCMD_SECTION_ERASE 0x14
#define NVMCMD_WORD_WRITE    0x1D

#endif
