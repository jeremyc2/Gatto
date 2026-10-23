#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef void (*GattoEvent)(void *, uint32_t);
void *GattoHotkeyCreate(const char *, uint32_t, uint32_t, GattoEvent, void *, int32_t *);
int32_t GattoHotkeyDestroy(void *);
void GattoHotkeyDetach(void *);
