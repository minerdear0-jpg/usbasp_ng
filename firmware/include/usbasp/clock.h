#ifndef USBASP_CLOCK_H_
#define USBASP_CLOCK_H_

#define TIMERVALUE TCNT0

#if F_CPU == 12000000L
# define CLOCK_T_320us 60
#elif F_CPU == 16000000L
# define CLOCK_T_320us 80
#elif F_CPU == 18000000L
# define CLOCK_T_320us 90
#elif F_CPU == 20000000L
# define CLOCK_T_320us 100
#else
# error "Unsupported F_CPU"
#endif

#if (defined __AVR_ATmega8__) || (defined __AVR_ATmega8A__)
#define TCCR0B TCCR0
#endif

#define clockInit() TCCR0B = (1 << CS01) | (1 << CS00)

void clockWait(uint8_t time);

#endif
