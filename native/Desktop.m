#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#import <CoreGraphics/CoreGraphics.h>
#import <IOKit/hidsystem/ev_keymap.h>
#include "Platform.h"

// Use the same open-document delegate as Launch Services. GPUI's callback only
// enqueues the selection, so service dispatch cannot reenter an active view borrow.
@interface GattoFileService : NSObject
- (void)sendToGatto:(NSPasteboard *)pasteboard userData:(NSString *)userData error:(NSString **)error;
@end

@implementation GattoFileService
- (void)sendToGatto:(NSPasteboard *)pasteboard userData:(NSString *)userData error:(NSString **)error {
    NSArray<NSURL *> *urls = [pasteboard readObjectsForClasses:@[NSURL.class]
        options:@{NSPasteboardURLReadingFileURLsOnlyKey: @YES}];
    if (!urls.count) {
        // Finder and older service clients can supply the legacy filename list.
        id paths = [pasteboard propertyListForType:NSFilenamesPboardType];
        NSMutableArray<NSURL *> *files = [NSMutableArray array];
        if ([paths isKindOfClass:NSArray.class]) {
            for (id path in paths) {
                if ([path isKindOfClass:NSString.class] && [path isAbsolutePath])
                    [files addObject:[NSURL fileURLWithPath:path]];
            }
        }
        urls = files;
    }
    if (!urls.count || ![NSApp.delegate respondsToSelector:@selector(application:openURLs:)]) {
        if (error) *error = @"Select local files in Finder and try Send to Gatto again.";
        return;
    }
    [NSApp.delegate application:NSApp openURLs:urls];
}
@end

void GattoInstallFileService(void) {
    assert(NSThread.isMainThread);
    static GattoFileService *provider;
    if (!provider) provider = [GattoFileService new];
    NSApp.servicesProvider = provider;
}

// Modifier bits belong to Gatto's ABI, not NSEvent or Carbon.
static NSEventModifierFlags eventModifiers(uint32_t modifiers) {
    return ((modifiers & 1) ? NSEventModifierFlagCommand : 0) |
           ((modifiers & 2) ? NSEventModifierFlagControl : 0) |
           ((modifiers & 4) ? NSEventModifierFlagOption : 0) |
           ((modifiers & 8) ? NSEventModifierFlagShift : 0);
}

@interface GattoStatusMenu : NSObject
@property NSStatusItem *status;
@property NSMenu *menu;
@property NSMenuItem *open;
@property NSMenuItem *preview;
@property NSMenuItem *quick;
@property NSMenuItem *preferences;
@property NSMenuItem *composition;
@property NSMenuItem *update;
@property NSMenuItem *quit;
@property BOOL pasteVisible;
@property BOOL updateVisible;
@property GattoEvent callback;
@property void *context;
@end

@implementation GattoStatusMenu
- (NSMenuItem *)item:(NSString *)title key:(NSString *)key tag:(NSInteger)tag {
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:@selector(activate:) keyEquivalent:key];
    item.target = self;
    item.tag = tag;
    item.keyEquivalentModifierMask = key.length ? NSEventModifierFlagCommand : 0;
    return item;
}
- (void)activate:(NSMenuItem *)item {
    if (self.callback) self.callback(self.context, (uint32_t)item.tag);
}
- (void)rebuild {
    [self.menu removeAllItems];
    [self.menu addItem:self.open];
    [self.menu addItem:self.composition];
    if (self.updateVisible) [self.menu addItem:self.update];
    if (self.pasteVisible) {
        [self.menu addItem:NSMenuItem.separatorItem];
        [self.menu addItem:self.preview];
        [self.menu addItem:self.quick];
    }
    [self.menu addItem:NSMenuItem.separatorItem];
    [self.menu addItem:self.preferences];
    [self.menu addItem:NSMenuItem.separatorItem];
    [self.menu addItem:self.quit];
}
@end

void *GattoMenuCreate(const uint8_t *bytes, size_t length, GattoEvent callback, void *context) {
    @autoreleasepool {
        if (!NSThread.isMainThread) return NULL;
        NSImage *image = [[NSImage alloc] initWithData:[NSData dataWithBytes:bytes length:length]];
        if (!image) return NULL;
        image.template = YES;
        image.size = NSMakeSize(18 * image.size.width / image.size.height, 18);
        GattoStatusMenu *owner = [GattoStatusMenu new];
        owner.callback = callback; owner.context = context;
        owner.menu = [NSMenu new];
        owner.menu.autoenablesItems = NO;
        owner.open = [owner item:@"Open App" key:@"" tag:1];
        owner.preview = [owner item:@"Draft from Clipboard" key:@"v" tag:2];
        owner.quick = [owner item:@"Quick Copy" key:@"" tag:3];
        owner.preferences = [owner item:@"App Preferences" key:@"," tag:4];
        owner.composition = [owner item:@"Side-by-Side Comparison" key:@"" tag:7];
        owner.update = [owner item:@"↑ Update" key:@"" tag:5];
        owner.quit = [owner item:@"Quit" key:@"q" tag:6];
        owner.status = [NSStatusBar.systemStatusBar statusItemWithLength:NSVariableStatusItemLength];
        if (!owner.status.button) return NULL;
        owner.status.button.image = image;
        owner.status.button.toolTip = @"Gatto";
        owner.status.menu = owner.menu;
        [owner rebuild];
        return (__bridge_retained void *)owner;
    }
}
void GattoMenuDestroy(void *handle) {
    if (!handle) return;
    GattoStatusMenu *owner = (__bridge_transfer GattoStatusMenu *)handle;
    owner.callback = NULL; owner.context = NULL;
    owner.status.menu = nil;
    [NSStatusBar.systemStatusBar removeStatusItem:owner.status];
}
void GattoMenuPasteVisible(void *handle, bool visible) {
    GattoStatusMenu *owner = (__bridge GattoStatusMenu *)handle;
    owner.pasteVisible = visible;
    [owner rebuild];
}
void GattoMenuUpdate(void *handle, uint32_t state, uint32_t identifier) {
    GattoStatusMenu *owner = (__bridge GattoStatusMenu *)handle;
    owner.updateVisible = state != 0;
    owner.update.tag = identifier;
    owner.update.title = state == 2 ? @"Updating…" : state == 3 ? @"↻ Retry Update" : @"↑ Update";
    owner.update.enabled = state != 2;
    [owner rebuild];
}
void GattoMenuShortcut(void *handle, const char *canonical, const char *equivalent, uint32_t modifiers, bool numpad) {
    GattoStatusMenu *owner = (__bridge GattoStatusMenu *)handle;
    NSString *key = [NSString stringWithUTF8String:equivalent];
    owner.quick.keyEquivalent = key;
    owner.quick.keyEquivalentModifierMask = key.length ? eventModifiers(modifiers) | (numpad ? NSEventModifierFlagNumericPad : 0) : 0;
    owner.quick.title = *canonical && !key.length
        ? [NSString stringWithFormat:@"Quick Copy (%s)", canonical] : @"Quick Copy";
}

// Physical ANSI/keypad codes are the SDK constants, independent of keyboard layout.
static int keyCode(const char *name) {
    static const struct { const char *name; int code; } keys[] = {
        {"KeyA",kVK_ANSI_A},{"KeyB",kVK_ANSI_B},{"KeyC",kVK_ANSI_C},{"KeyD",kVK_ANSI_D},
        {"KeyE",kVK_ANSI_E},{"KeyF",kVK_ANSI_F},{"KeyG",kVK_ANSI_G},{"KeyH",kVK_ANSI_H},
        {"KeyI",kVK_ANSI_I},{"KeyJ",kVK_ANSI_J},{"KeyK",kVK_ANSI_K},{"KeyL",kVK_ANSI_L},
        {"KeyM",kVK_ANSI_M},{"KeyN",kVK_ANSI_N},{"KeyO",kVK_ANSI_O},{"KeyP",kVK_ANSI_P},
        {"KeyQ",kVK_ANSI_Q},{"KeyR",kVK_ANSI_R},{"KeyS",kVK_ANSI_S},{"KeyT",kVK_ANSI_T},
        {"KeyU",kVK_ANSI_U},{"KeyV",kVK_ANSI_V},{"KeyW",kVK_ANSI_W},{"KeyX",kVK_ANSI_X},
        {"KeyY",kVK_ANSI_Y},{"KeyZ",kVK_ANSI_Z},
        {"Digit0",kVK_ANSI_0},{"Digit1",kVK_ANSI_1},{"Digit2",kVK_ANSI_2},{"Digit3",kVK_ANSI_3},
        {"Digit4",kVK_ANSI_4},{"Digit5",kVK_ANSI_5},{"Digit6",kVK_ANSI_6},{"Digit7",kVK_ANSI_7},
        {"Digit8",kVK_ANSI_8},{"Digit9",kVK_ANSI_9},
        {"Backquote",kVK_ANSI_Grave},{"Backslash",kVK_ANSI_Backslash},{"BracketLeft",kVK_ANSI_LeftBracket},
        {"BracketRight",kVK_ANSI_RightBracket},{"Comma",kVK_ANSI_Comma},{"Equal",kVK_ANSI_Equal},
        {"Minus",kVK_ANSI_Minus},{"Period",kVK_ANSI_Period},{"Quote",kVK_ANSI_Quote},
        {"Semicolon",kVK_ANSI_Semicolon},{"Slash",kVK_ANSI_Slash},{"IntlBackslash",kVK_ISO_Section},
        {"Enter",kVK_Return},{"Tab",kVK_Tab},{"Space",kVK_Space},{"Backspace",kVK_Delete},
        {"Delete",kVK_ForwardDelete},{"Escape",kVK_Escape},{"CapsLock",kVK_CapsLock},
        {"ArrowLeft",kVK_LeftArrow},{"ArrowRight",kVK_RightArrow},{"ArrowUp",kVK_UpArrow},{"ArrowDown",kVK_DownArrow},
        {"Home",kVK_Home},{"End",kVK_End},{"PageUp",kVK_PageUp},{"PageDown",kVK_PageDown},{"Insert",kVK_Help},
        {"Numpad0",kVK_ANSI_Keypad0},{"Numpad1",kVK_ANSI_Keypad1},{"Numpad2",kVK_ANSI_Keypad2},
        {"Numpad3",kVK_ANSI_Keypad3},{"Numpad4",kVK_ANSI_Keypad4},{"Numpad5",kVK_ANSI_Keypad5},
        {"Numpad6",kVK_ANSI_Keypad6},{"Numpad7",kVK_ANSI_Keypad7},{"Numpad8",kVK_ANSI_Keypad8},{"Numpad9",kVK_ANSI_Keypad9},
        {"NumpadAdd",kVK_ANSI_KeypadPlus},{"NumpadSubtract",kVK_ANSI_KeypadMinus},{"NumpadMultiply",kVK_ANSI_KeypadMultiply},
        {"NumpadDivide",kVK_ANSI_KeypadDivide},{"NumpadDecimal",kVK_ANSI_KeypadDecimal},{"NumpadEnter",kVK_ANSI_KeypadEnter},
        {"NumpadEqual",kVK_ANSI_KeypadEquals},{"NumLock",kVK_ANSI_KeypadClear},
        {"AudioVolumeUp",kVK_VolumeUp},{"AudioVolumeDown",kVK_VolumeDown},{"AudioVolumeMute",kVK_Mute},
        {"PrintScreen",0x46}, // Preserve the previous macOS backend's mapping.
        {"F1",kVK_F1},{"F2",kVK_F2},{"F3",kVK_F3},{"F4",kVK_F4},{"F5",kVK_F5},
        {"F6",kVK_F6},{"F7",kVK_F7},{"F8",kVK_F8},{"F9",kVK_F9},{"F10",kVK_F10},
        {"F11",kVK_F11},{"F12",kVK_F12},{"F13",kVK_F13},{"F14",kVK_F14},{"F15",kVK_F15},
        {"F16",kVK_F16},{"F17",kVK_F17},{"F18",kVK_F18},{"F19",kVK_F19},{"F20",kVK_F20}
    };
    for (size_t i=0; i<sizeof(keys)/sizeof(keys[0]); ++i) if (!strcmp(name,keys[i].name)) return keys[i].code;
    return -1;
}

@interface GattoHotkey : NSObject {
@public
    EventHotKeyRef hotkey;
    EventHandlerRef handler;
    CFMachPortRef tap;
    CFRunLoopSourceRef source;
}
@property GattoEvent callback;
@property void *context;
@property uint32_t identifier;
@property uint32_t modifiers;
@property int media;
@end
@implementation GattoHotkey
@end

static OSStatus hotkeyEvent(EventHandlerCallRef next, EventRef event, void *context) {
    GattoHotkey *owner = (__bridge GattoHotkey *)context;
    EventHotKeyID identifier;
    OSStatus result = GetEventParameter(event,kEventParamDirectObject,typeEventHotKeyID,NULL,sizeof(identifier),NULL,&identifier);
    if (result != noErr || identifier.signature != 0x47747448 || identifier.id != owner.identifier || !owner.callback) return eventNotHandledErr;
    owner.callback(owner.context,owner.identifier);
    return noErr;
}
static CGEventRef mediaEvent(CGEventTapProxy proxy, CGEventType type, CGEventRef event, void *context) {
    GattoHotkey *owner = (__bridge GattoHotkey *)context;
    if (type == kCGEventTapDisabledByTimeout) {
        if (owner.callback) CGEventTapEnable(owner->tap,true);
        return event;
    }
    if (type != NX_SYSDEFINED || !owner.callback) return event;
    NSEvent *native = [NSEvent eventWithCGEvent:event];
    if (native.subtype != 8) return event;
    NSInteger data = native.data1;
    NSEventModifierFlags mask = NSEventModifierFlagCommand | NSEventModifierFlagControl | NSEventModifierFlagOption | NSEventModifierFlagShift;
    if (((data >> 16) & 0xffff) != owner.media || (native.modifierFlags & mask) != eventModifiers(owner.modifiers)) return event;
    if (((data >> 8) & 0xff) == 0x0a) owner.callback(owner.context,owner.identifier);
    return NULL; // Suppress only matching media-key presses/releases.
}
void *GattoHotkeyCreate(const char *key, uint32_t modifiers, uint32_t identifier,
                       GattoEvent callback, void *context, int32_t *error) {
    @autoreleasepool {
        *error = paramErr;
        if (!NSThread.isMainThread) return NULL;
        GattoHotkey *owner = [GattoHotkey new];
        owner.callback=callback; owner.context=context; owner.identifier=identifier; owner.modifiers=modifiers;
        owner.media = !strcmp(key,"MediaPlayPause") ? NX_KEYTYPE_PLAY : !strcmp(key,"MediaTrackNext") ? NX_KEYTYPE_NEXT : !strcmp(key,"MediaTrackPrevious") ? NX_KEYTYPE_PREVIOUS : -1;
        if (owner.media >= 0) {
            owner->tap = CGEventTapCreate(kCGSessionEventTap,kCGHeadInsertEventTap,kCGEventTapOptionDefault,CGEventMaskBit(NX_SYSDEFINED),mediaEvent,(__bridge void *)owner);
            if (!owner->tap) { *error = -1; return NULL; }
            owner->source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault,owner->tap,0);
            if (!owner->source) { CFMachPortInvalidate(owner->tap); CFRelease(owner->tap); *error=-1; return NULL; }
            CFRunLoopAddSource(CFRunLoopGetMain(),owner->source,kCFRunLoopCommonModes);
            CGEventTapEnable(owner->tap,true);
        } else {
            int code = keyCode(key);
            if (code < 0) return NULL;
            EventTypeSpec pressed = {kEventClassKeyboard,kEventHotKeyPressed};
            *error = InstallApplicationEventHandler(hotkeyEvent,1,&pressed,(__bridge void *)owner,&owner->handler);
            if (*error != noErr) return NULL;
            UInt32 native = ((modifiers&1)?cmdKey:0)|((modifiers&2)?controlKey:0)|((modifiers&4)?optionKey:0)|((modifiers&8)?shiftKey:0);
            *error = RegisterEventHotKey((UInt32)code,native,(EventHotKeyID){0x47747448,identifier},GetApplicationEventTarget(),0,&owner->hotkey);
            if (*error != noErr) {
                if (RemoveEventHandler(owner->handler) != noErr) {
                    owner.callback = NULL; owner.context = NULL;
                    (void)CFBridgingRetain(owner); // OS still owns this detached context.
                }
                return NULL;
            }
        }
        *error = noErr;
        return (__bridge_retained void *)owner;
    }
}
void GattoHotkeyDetach(void *handle) {
    GattoHotkey *owner = (__bridge GattoHotkey *)handle;
    owner.callback = NULL; owner.context = NULL;
}
int32_t GattoHotkeyDestroy(void *handle) {
    GattoHotkey *owner = (__bridge GattoHotkey *)handle;
    if (owner->hotkey) {
        OSStatus status = UnregisterEventHotKey(owner->hotkey);
        if (status != noErr) return status; // Caller retains the old registration.
        owner->hotkey = NULL;
    }
    GattoHotkeyDetach(handle);
    if (owner->handler && RemoveEventHandler(owner->handler) != noErr) return noErr; // Leak detached owner rather than leave dangling userData.
    if (owner->source) {
        CFRunLoopRemoveSource(CFRunLoopGetMain(),owner->source,kCFRunLoopCommonModes);
        CFRelease(owner->source);
    }
    if (owner->tap) { CFMachPortInvalidate(owner->tap); CFRelease(owner->tap); }
    (void)CFBridgingRelease(handle);
    return noErr;
}
