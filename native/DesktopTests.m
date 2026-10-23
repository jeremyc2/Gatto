// Exercise the real AppKit menu and native event dispatch without sending keys
// or changing any user's global shortcut registrations.
#include "Desktop.m"
#include <assert.h>

static void received(void *context, uint32_t event) { *(uint32_t *)context = event; }
@interface FileReceiver : NSObject <NSApplicationDelegate>
@property NSArray<NSURL *> *selection;
@end
@implementation FileReceiver
- (void)application:(NSApplication *)application openURLs:(NSArray<NSURL *> *)urls {
    self.selection = urls;
}
@end

int main(int argc, const char **argv) {
    @autoreleasepool {
        assert(argc == 2);
        [NSApplication sharedApplication];
        FileReceiver *receiver = [FileReceiver new];
        NSApp.delegate = receiver;
        GattoInstallFileService();
        GattoFileService *service = NSApp.servicesProvider;
        NSPasteboard *board = [NSPasteboard pasteboardWithUniqueName];
        NSArray<NSURL *> *selection = @[[NSURL fileURLWithPath:@"/tmp/猫 #?%\n$(touch nope).png"],
                                      [NSURL fileURLWithPath:@"/tmp/video clip.mov"]];
        [board writeObjects:selection];
        NSString *error = nil;
        [service sendToGatto:board userData:nil error:&error];
        assert(!error && [receiver.selection isEqualToArray:selection]);
        // Finder services can supply file-reference URLs. GPUI serializes these
        // as strings, so the service must resolve them before handing them off.
        NSString *fixture = [NSTemporaryDirectory() stringByAppendingPathComponent:
            [NSString stringWithFormat:@"gatto-service-%@ 猫 #?.png", NSUUID.UUID.UUIDString]];
        assert([[NSData dataWithBytes:"image" length:5] writeToFile:fixture atomically:YES]);
        NSURL *file = [NSURL fileURLWithPath:fixture];
        NSURL *reference = file.fileReferenceURL;
        assert(reference.isFileReferenceURL);
        [board clearContents];
        [board writeObjects:@[reference, selection.lastObject]];
        receiver.selection = nil;
        [service sendToGatto:board userData:nil error:&error];
        NSString *expectedURL = reference.filePathURL.absoluteString;
        assert([NSFileManager.defaultManager removeItemAtPath:fixture error:nil]);
        assert(!error && receiver.selection.count == 2);
        NSURL *delivered = receiver.selection.firstObject;
        assert(!delivered.isFileReferenceURL && "Service forwarded a /.file/id= reference instead of a path URL");
        assert([delivered.absoluteString isEqualToString:expectedURL]);
        assert([receiver.selection.lastObject isEqual:selection.lastObject]);
        // A stale Finder reference should report an actionable service error,
        // never pass the synthetic /.file/id= path to the importer.
        [board clearContents];
        [board writeObjects:@[[NSURL URLWithString:reference.absoluteString]]];
        receiver.selection = nil;
        [service sendToGatto:board userData:nil error:&error];
        assert(error && !receiver.selection);
        error = nil;
        [board clearContents];
        [board setPropertyList:[selection valueForKey:@"path"] forType:NSFilenamesPboardType];
        receiver.selection = nil;
        [service sendToGatto:board userData:nil error:&error];
        assert(!error && [receiver.selection isEqualToArray:selection]);
        [board clearContents];
        [board writeObjects:@[[NSURL URLWithString:@"https://example.com/a.png"]]];
        receiver.selection = nil;
        [service sendToGatto:board userData:nil error:&error];
        assert(error && !receiver.selection);
        [board releaseGlobally];
        NSApp.servicesProvider = nil;
        NSApp.delegate = nil;
        NSData *png = [NSData dataWithContentsOfFile:[NSString stringWithUTF8String:argv[1]]];
        uint32_t event = 0;
        void *handle = GattoMenuCreate(png.bytes,png.length,received,&event);
        assert(handle);
        GattoStatusMenu *owner = (__bridge GattoStatusMenu *)handle;
        assert(owner.status.button.image.isTemplate);
        assert(owner.menu.numberOfItems == 6);
        GattoMenuPasteVisible(handle,true);
        assert(owner.menu.numberOfItems == 9);
        assert([owner.preview.title isEqualToString:@"Draft from Clipboard"]);
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
        assert(!owner.update.isEnabled && owner.menu.numberOfItems == 10);
        GattoMenuPasteVisible(handle,false);
        assert(owner.menu.numberOfItems == 7 && [owner.menu itemAtIndex:2] == owner.update);
        assert([owner.composition.title isEqualToString:@"Side-by-Side Comparison"]);
        [owner activate:owner.composition];
        assert(event == 7);
        [owner activate:owner.preferences];
        assert(event == 4);
        GattoMenuUpdate(handle,0,517);
        assert(owner.update.tag == 517);
        [owner activate:owner.update];
        assert(event == 517);
        assert(owner.menu.numberOfItems == 6);
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
        puts("Native file service, menu lifecycle, shortcuts and event dispatch passed.");
    }
}
