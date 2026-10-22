// Exercise the real AppKit menu and native event dispatch without sending keys
// or changing any user's global shortcut registrations.
#include "Desktop.m"
#include <assert.h>

static void received(void *context, uint32_t event) { *(uint32_t *)context = event; }
int main(int argc, const char **argv) {
    @autoreleasepool {
        assert(argc == 2);
        [NSApplication sharedApplication];
        NSData *png = [NSData dataWithContentsOfFile:[NSString stringWithUTF8String:argv[1]]];
        uint32_t event = 0;
        void *handle = GattoMenuCreate(png.bytes,png.length,received,&event);
        assert(handle);
        GattoStatusMenu *owner = (__bridge GattoStatusMenu *)handle;
        assert(owner.status.button.image.isTemplate);
        assert(owner.menu.numberOfItems == 5);
        GattoMenuPasteVisible(handle,true);
        assert(owner.menu.numberOfItems == 8);
        assert([owner.preview.title isEqualToString:@"Stage from Clipboard"]);
        assert([owner.preview.keyEquivalent isEqualToString:@"v"]);
        GattoMenuShortcut(handle,"Command+Shift+Numpad1","1",9,true);
        assert(owner.quick.keyEquivalentModifierMask & NSEventModifierFlagNumericPad);
        GattoMenuShortcut(handle,"","",0,false);
        assert(owner.quick.keyEquivalent.length == 0);
        assert(owner.quick.keyEquivalentModifierMask == 0);
        GattoMenuShortcut(handle,"Command+Shift+MediaPlayPause","",9,false);
        assert([owner.quick.title containsString:@"MediaPlayPause"]);
        GattoMenuUpdate(handle,2,261);
        assert(owner.update.tag == 261);
        assert(!owner.update.isEnabled && owner.menu.numberOfItems == 9);
        GattoMenuPasteVisible(handle,false);
        assert(owner.menu.numberOfItems == 6 && [owner.menu itemAtIndex:1] == owner.update);
        [owner activate:owner.preferences];
        assert(event == 4);
        GattoMenuUpdate(handle,0,517);
        assert(owner.update.tag == 517);
        [owner activate:owner.update];
        assert(event == 517);
        assert(owner.menu.numberOfItems == 5);
        GattoMenuDestroy(handle);

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
        puts("Native menu lifecycle, shortcuts and event dispatch passed.");
    }
}
