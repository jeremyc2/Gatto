#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef intptr_t (*GattoRead)(void *, uint8_t *, size_t);
typedef bool (*GattoCanceled)(void *);
typedef void (*GattoProgress)(void *, uint64_t, uint64_t);
typedef struct {
    uint16_t status;
    int error;
    uint8_t *body;
    size_t body_length;
    char *retry_after;
} GattoUploadResult;

// Synchronous worker entry. All delegate callbacks finish before this returns.
void GattoUpload(const char *url, const char *token, const char *mime,
                 const char *agent, uint64_t length, double total, double idle,
                 void *context, GattoRead read, GattoCanceled canceled,
                 GattoProgress progress, GattoUploadResult *result);
void GattoUploadResultFree(GattoUploadResult *result);

typedef void (*GattoEvent)(void *, uint32_t);
void GattoInstallFileService(void);
void *GattoMenuCreate(const uint8_t *, size_t, GattoEvent, void *);
void GattoMenuDestroy(void *);
void GattoMenuPasteVisible(void *, bool);
void GattoMenuUpdate(void *, uint32_t, uint32_t);
void GattoMenuShortcut(void *, const char *, const char *, uint32_t, bool);
void *GattoHotkeyCreate(const char *, uint32_t, uint32_t, GattoEvent, void *, int32_t *);
int32_t GattoHotkeyDestroy(void *);
void GattoHotkeyDetach(void *);
