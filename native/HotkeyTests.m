// The small Carbon exception: validate signatures and dispatch without sending keys.
#import <Carbon/Carbon.h>
static OSStatus unregisterResult = noErr, removeResult = noErr;
static unsigned unregisterCalls = 0, removeCalls = 0;
static OSStatus fakeUnregister(EventHotKeyRef key) { ++unregisterCalls; return unregisterResult; }
static OSStatus fakeRemove(EventHandlerRef handler) { ++removeCalls; return removeResult; }
#define UnregisterEventHotKey fakeUnregister
#define RemoveEventHandler fakeRemove
#include "Hotkey.m"
#include <assert.h>
static void received(void *context, uint32_t event) { *(uint32_t *)context = event; }
int main(void) {
    @autoreleasepool {
        uint32_t event = 0;
        GattoHotkey *key = [GattoHotkey new];
        key.identifier = 42; key.callback = received; key.context = &event;
        EventRef native;
        assert(CreateEvent(NULL,kEventClassKeyboard,kEventHotKeyPressed,0,0,&native) == noErr);
        EventHotKeyID identifier = {0x12345678,42};
        SetEventParameter(native,kEventParamDirectObject,typeEventHotKeyID,sizeof(identifier),&identifier);
        assert(hotkeyEvent(NULL,native,(__bridge void *)key) == eventNotHandledErr);
        identifier.signature = 0x47747448;
        SetEventParameter(native,kEventParamDirectObject,typeEventHotKeyID,sizeof(identifier),&identifier);
        assert(hotkeyEvent(NULL,native,(__bridge void *)key) == noErr);
        assert(event == 42);
        ReleaseEvent(native);
        assert(mediaEvent(NULL,kCGEventTapDisabledByUserInput,NULL,(__bridge void *)key) == NULL);
        // OS failure must leave the old registration/callback alive until the
        // Rust owner explicitly detaches. The fake refs never reach Carbon.
        key->hotkey = (EventHotKeyRef)1;
        key->handler = (EventHandlerRef)2;
        void *handle = (__bridge_retained void *)key;
        unregisterResult = paramErr;
        assert(GattoHotkeyDestroy(handle) == paramErr);
        assert(key->hotkey && key.callback && key.context == &event);
        assert(unregisterCalls == 1 && removeCalls == 0);
        GattoHotkeyDetach(handle);
        assert(!key.callback && !key.context);
        unregisterResult = noErr;
        assert(GattoHotkeyDestroy(handle) == noErr);
        assert(!key->hotkey && unregisterCalls == 2 && removeCalls == 1);

        // A failed handler removal deliberately retains a detached native owner.
        // Test its lifetime, then release the test-only retained reference.
        __weak GattoHotkey *weakOrphan;
        @autoreleasepool {
        GattoHotkey *orphan = [GattoHotkey new];
        orphan->handler = (EventHandlerRef)2;
        orphan.callback = received; orphan.context = &event;
        void *orphanHandle = (__bridge_retained void *)orphan;
        weakOrphan = orphan;
        removeResult = paramErr;
        assert(GattoHotkeyDestroy(orphanHandle) == noErr);
        assert(!orphan.callback && !orphan.context);
        orphan = nil;
        assert(weakOrphan && "Failed cleanup freed the OS callback context");
        (void)CFBridgingRelease(orphanHandle); // No real OS registration in this test.
        }
        assert(!weakOrphan);
        puts("Carbon dispatch, unregister failure and detached-owner lifetime passed.");
    }
}
